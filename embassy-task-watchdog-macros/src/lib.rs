use core::sync::atomic::{AtomicU32, Ordering};

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, FnArg, Ident, ItemFn, MetaNameValue, Pat, PatIdent, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

#[derive(Clone)]
struct TaskArgs {
    docs: Vec<syn::Attribute>,
    timeout: Expr,
    keep: bool,
    setup: bool,
    fallible: bool,
}

#[cfg(not(debug_assertions))]
use embassy_task_watchdog_numtasks::MAX_TASKS;
static TASK_ID: AtomicU32 = AtomicU32::new(0);

struct ParseResult {
    vis: syn::Visibility,
    sig: syn::Signature,
    fn_ident: Ident,
    broken: Option<(Vec<syn::Stmt>, Vec<syn::Stmt>)>,
    block: Box<syn::Block>,
    wd_ident: Ident,
    desc_id: u32,
    max_expr: Expr,
}

impl Parse for TaskArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut timeout: Option<Expr> = None;
        let mut keep = true;
        let mut setup = false;
        let mut fallible = false;
        let docs = input.call(syn::Attribute::parse_outer)?;
        let docs = docs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .cloned()
            .collect::<Vec<_>>();
        let args: Punctuated<MetaNameValue, Token![,]> =
            input.parse_terminated(MetaNameValue::parse, Token![,])?;

        for nv in args {
            let key = nv.path.get_ident().map(|i| i.to_string());
            match key.as_deref() {
                Some("timeout") => timeout = Some(nv.value),
                Some("keep") => {
                    keep = check_boolean_expr(&nv.value, "keep")?;
                }
                Some("setup") => {
                    setup = check_boolean_expr(&nv.value, "setup")?;
                }
                Some("fallible") => {
                    fallible = check_boolean_expr(&nv.value, "fallible")?;
                }
                Some(other) => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        format!(
                            "unknown argument `{other}` (supported: timeout, keep, setup, fallible)"
                        ),
                    ));
                }
                None => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        "expected identifier key (supported: timeout)",
                    ));
                }
            }
        }

        Ok(Self {
            docs,
            timeout: timeout.ok_or_else(|| {
                syn::Error::new(input.span(), "missing required: timeout = <expr>")
            })?,
            keep,
            setup,
            fallible,
        })
    }
}

fn first_param_ident(fn_item: &ItemFn) -> Result<Ident> {
    let first = fn_item.sig.inputs.iter().next().ok_or_else(|| {
        syn::Error::new(
            fn_item.sig.span(),
            "task function must have at least one parameter (watchdog runner reference)",
        )
    })?;

    match first {
        FnArg::Receiver(_) => Err(syn::Error::new(
            first.span(),
            "first parameter must be an identifier pattern",
        )),
        FnArg::Typed(pat_ty) => match &*pat_ty.pat {
            Pat::Ident(PatIdent { ident, .. }) => Ok(ident.clone()),
            other => Err(syn::Error::new(
                other.span(),
                "first parameter must be like `wd: ...`",
            )),
        },
    }
}

#[proc_macro_attribute]
/// This decorator macro replaces [`embassy_executor::task`](https://docs.embassy.dev/embassy-executor/git/cortex-m/attr.task.html)
/// to create an async task that can be monitored by the task-watchdog.  
///
/// # Arguments
/// - `timeout`: The duration to wait for a feed before considering the task stalled (e.g. `timeout = Duration::from_millis(2000)`)
/// - `setup`: Whether this task contains setup code before the main loop (*defaults to `false`*). If `true`, the macro will split the
///   function body into a setup part (before the first loop) and a consume part (from the first loop onward), and will only
///   register the watchdog and apply the timeout to the consume part. This allows for longer-running setup code without
///   triggering a false positive stall detection, while still monitoring the main loop of the task. A task with `setup = true`
///   must contain at least one loop statement (e.g. `loop { ... }`) for the macro to split on, and the user is expected to
///   feed the watchdog inside the loop(s) to indicate the task is still alive.
/// - `keep`: Whether to keep the task descriptor after the task finishes (*defaults to `true`*).
///   If `keep` is `false`, the task will be deregistered upon completion. A task with `keep = true` _should_ not be fallible.
/// - `fallible`: Setting this to `true` will remove the requirement for the task function to return `!` (never), and allow it to
///   return normally. This is useful for tasks that may need to exit on their own after some condition is met, rather than running
///   indefinitely. A fallible task will still be monitored by the watchdog until it finishes, and crash the program. Set
///   `keep = false` for a fallible task to deregister itself from the watchdog when it finishes.
///
/// Example usage for a task
/// is shown below, that feeds the watchdog every 1000ms and is considered stalled if it goes more
/// than 2000ms without feeding:
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::{Duration, Timer};
/// #[task(timeout = Duration::from_millis(2000))]
/// async fn my_task(watchdog: TaskWatchdog) {
///     loop {
///       watchdog.feed().await; // Feed the watchdog to indicate the task is still alive
///        // Do some work
///        Timer::after(Duration::from_millis(1000)).await;
///     }
/// }
/// ```
/// The first argument to the task must be a [`embassy_task_watchdog::TaskWatchdog`](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpTaskWatchdog.html)
/// for the task to register itself with. The macro will convert this into a per-task bound watchdog
///  [`embassy_task_watchdog::BoundWatchdog`](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpBoundWatchdog.html)
/// that the user can feed to indicate the task is still alive. The `timeout` argument specifies how
/// long the watchdog should wait for a feed before considering the task to be stalled. This value
/// should be set to be longer than the longest expected time between feeds in the task.
///
/// Example usage for a task with setup code is shown below:
/// ```rust,no_run
/// #[task(timeout = Duration::from_millis(2000), setup = true)]
/// async fn my_task_with_setup(watchdog: TaskWatchdog) {
///     // Some setup code that runs once
///     do_setup().await;
///     loop {
///         watchdog.feed().await; // Feed the watchdog to indicate the task is still alive
///         // Do some work
///     }
/// }
/// ```
///
/// Example usage for a fallible task is shown below:
/// ```rust,no_run
/// #[task(timeout = Duration::from_millis(2000), fallible = true, keep = false)]
/// async fn my_fallible_task(watchdog: TaskWatchdog) {
///     loop {
///         watchdog.feed().await; // Feed the watchdog to indicate the task is still alive
///         // Do some work
///         if some_condition() {
///             return; // Exit the task normally, which will deregister it from the watchdog since keep = false
///         }
///     }
///     // task is deregistered here, since keep = false, so the watchdog will stop monitoring it
///     // and won't trigger a system reset.
/// }
/// ```
///
/// # Caution
/// In release builds, the macro checks that the number of tasks does not exceed the configured limit
/// (defaults to 32), and will produce a compile error if more tasks are defined. In debug builds, this
/// check is skipped to allow for continuous integration testing without needing to adjust the limit.
pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input attributes into a syntax tree, as TaskArgs
    let args = parse_macro_input!(attr as TaskArgs);
    // Parse the input item into a syntax tree, as a function
    let f = parse_macro_input!(item as ItemFn);
    let result = match analyze_task(&args, f) {
        Ok(result) => result,
        Err(e) => return e.to_compile_error().into(),
    };
    let vis = result.vis;
    let sig = result.sig;
    let fn_ident = result.fn_ident;
    let broken = result.broken;
    let block = result.block;
    let wd_ident = result.wd_ident;
    let desc_id = result.desc_id;
    let max_expr = result.max_expr;
    let docs = args.docs;
    let docs = if !docs.is_empty() {
        quote! { #(#docs)* }
    } else {
        quote! {}
    };

    let body = if let Some((setup, consume)) = broken {
        quote! {
            // Setup code before the loop, which is not monitored by the watchdog
            #(#setup)*
            // Register the watchdog after the setup code finishes
            let #wd_ident = #wd_ident._register_desc(::core::stringify!(#fn_ident), #desc_id, #max_expr).await;
            // Run the loop code, which has access to the bound watchdog for feeding, and is monitored for stalls
            #(#consume)*
        }
    } else {
        quote! {
            // Convert runner ref into a per-task bound watchdog
            let #wd_ident = #wd_ident._register_desc(::core::stringify!(#fn_ident), #desc_id, #max_expr).await;
            {
                // Run user code
                #block
            }
        }
    };

    let body = if !args.keep {
        quote! {
            #body
            // Deregister the watchdog when the task finishes, since it won't be around to feed it anymore
            #wd_ident._deregister().await;
        }
    } else {
        body
    };

    // Generate the output tokens for the task function, which includes:
    // - A static task descriptor with the unique ID and function name
    // - Registering the watchdog runner reference as a bound watchdog with the descriptor and timeout
    // - The user body of the function, which now has access to the bound watchdog for feeding
    let expanded = quote! {
        #docs
        #[embassy_executor::task]
        #vis #sig {
            #body
        }
    };

    expanded.into()
}

fn analyze_task(args: &TaskArgs, f: ItemFn) -> Result<ParseResult> {
    // Check function is async
    if f.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            f.sig.span(),
            "#[embassy_task_watchdog::task] must be on async fn",
        ));
    }
    // Check function returns !
    if !args.fallible {
        match f.sig.output {
            syn::ReturnType::Default => {
                return Err(syn::Error::new(
                    f.sig.output.span(),
                    "#[embassy_task_watchdog::task] function must return ! (never)",
                ));
            }
            syn::ReturnType::Type(_, ref ty) => {
                if ty.to_token_stream().to_string() != "!" {
                    return Err(syn::Error::new(
                        ty.span(),
                        "#[embassy_task_watchdog::task] function must return ! (never)",
                    ));
                }
            }
        }
    }
    // Check function contains at least one loop, and split the statements into setup and loop parts
    // Get the original statement body of the function block
    let broken = if args.setup {
        let original_statements = f.block.stmts.clone();
        let mut indices = Vec::new();
        // loop through the statements and find the indices of any loop statements
        for (index, stmt) in original_statements.iter().enumerate() {
            if let syn::Stmt::Expr(expr, _) = stmt
                && let Expr::Loop(_) = expr
            {
                indices.push(index)
            }
        }
        // If no loop statements were found, return a compile error
        if indices.is_empty() {
            return Err(syn::Error::new(
                f.block.span(),
                "#[embassy_task_watchdog::task] function must contain at least one loop to allow for feeding the watchdog (e.g. `loop { ... }`)",
            ));
        }
        // Split the original statements into setup statements (before the first loop) and consume statements (from the first loop onward)
        let (setup, consume) = original_statements.split_at(indices[0]);
        Some((setup.to_vec(), consume.to_vec()))
    } else {
        None
    };
    // Get the identifier of the first parameter, which will be used as the watchdog runner reference
    let wd_ident = first_param_ident(&f)?;
    // Generate a unique descriptor ID for this task, and check against the limit in release builds
    let desc_id = TASK_ID.fetch_add(1, Ordering::SeqCst);
    #[cfg(not(debug_assertions))]
    if desc_id >= MAX_TASKS as _ {
        return Err(syn::Error::new(
            f.sig.span(),
            format!(
                "task limit exceeded ({desc_id}): max is {MAX_TASKS} (consider increasing the limit in embassy-task-watchdog-numtasks)"
            ),
        ));
    }
    // Extract the function visibility, signature, identifier, and body block for later use in code generation
    let vis = &f.vis;
    let sig = &f.sig;
    let fn_ident = &f.sig.ident;
    let block = &f.block;
    // Extract the timeout expression from the macro arguments for later use in code generation
    let max_expr = args.timeout.clone();
    Ok(ParseResult {
        vis: vis.clone(),
        sig: sig.clone(),
        fn_ident: fn_ident.clone(),
        broken,
        block: block.clone(),
        wd_ident: wd_ident.clone(),
        desc_id,
        max_expr,
    })
}

fn check_boolean_expr(expr: &Expr, name: &str) -> Result<bool> {
    if let Expr::Lit(expr_lit) = expr
        && let syn::Lit::Bool(lit_bool) = &expr_lit.lit
    {
        Ok(lit_bool.value)
    } else {
        Err(syn::Error::new(
            expr.span(),
            format!("expected boolean literal (e.g. {name} = true or {name} = false)"),
        ))
    }
}
