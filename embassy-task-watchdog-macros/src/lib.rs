use core::sync::atomic::{AtomicU32, Ordering};

use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Expr, FnArg, Ident, ItemFn, MetaNameValue, Pat, PatIdent, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

struct TaskArgs {
    timeout: Expr,
}

#[cfg(not(debug_assertions))]
use embassy_task_watchdog_numtasks::MAX_TASKS;
static TASK_ID: AtomicU32 = AtomicU32::new(0);

impl Parse for TaskArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut timeout: Option<Expr> = None;
        let args: Punctuated<MetaNameValue, Token![,]> =
            input.parse_terminated(MetaNameValue::parse, Token![,])?;

        for nv in args {
            let key = nv.path.get_ident().map(|i| i.to_string());
            match key.as_deref() {
                Some("timeout") => timeout = Some(nv.value),
                Some(other) => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        format!("unknown argument `{other}` (supported: timeout)"),
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
            timeout: timeout.ok_or_else(|| {
                syn::Error::new(input.span(), "missing required: timeout = <expr>")
            })?,
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
/// to create an async task that can be monitored by the task-watchdog.  Example usage for a task
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
/// # Caution
/// In release builds, the macro checks that the number of tasks does not exceed the configured limit
/// (defaults to 32), and will produce a compile error if more tasks are defined. In debug builds, this
/// check is skipped to allow for continuous integration testing without needing to adjust the limit.
pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input attributes into a syntax tree, as TaskArgs
    let args = parse_macro_input!(attr as TaskArgs);
    // Parse the input item into a syntax tree, as a function
    let f = parse_macro_input!(item as ItemFn);

    // Check function is async
    if f.sig.asyncness.is_none() {
        return syn::Error::new(
            f.sig.span(),
            "#[embassy_task_watchdog::task] must be on async fn",
        )
        .to_compile_error()
        .into();
    }
    // Check function returns !
    match f.sig.output {
        syn::ReturnType::Default => {
            return syn::Error::new(
                f.sig.output.span(),
                "#[embassy_task_watchdog::task] function must return ! (never)",
            )
            .to_compile_error()
            .into();
        }
        syn::ReturnType::Type(_, ref ty) => {
            if ty.to_token_stream().to_string() != "!" {
                return syn::Error::new(
                    ty.span(),
                    "#[embassy_task_watchdog::task] function must return ! (never)",
                )
                .to_compile_error()
                .into();
            }
        }
    }
    // Get the identifier of the first parameter, which will be used as the watchdog runner reference
    let wd_ident = match first_param_ident(&f) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error().into(),
    };
    // Generate a unique descriptor ID for this task, and check against the limit in release builds
    let desc_id = TASK_ID.fetch_add(1, Ordering::SeqCst);
    #[cfg(not(debug_assertions))]
    if desc_id >= MAX_TASKS as _ {
        return syn::Error::new(
            f.sig.span(),
            format!(
                "task limit exceeded ({desc_id}): max is {MAX_TASKS} (consider increasing the limit in embassy-task-watchdog-numtasks)"
            ),
        )
        .to_compile_error()
        .into();
    }
    // Extract the function visibility, signature, identifier, and body block for later use in code generation
    let vis = &f.vis;
    let sig = &f.sig;
    let fn_ident = &f.sig.ident;
    let block = &f.block;
    // Create a unique identifier for the task descriptor static variable, based on the function name
    let desc_ident = format_ident!(
        "__EMBASSY_TASK_WATCHDOG_DESC_{}",
        fn_ident.to_string().to_uppercase()
    );
    // Extract the timeout expression from the macro arguments for later use in code generation
    let max_expr = args.timeout;
    // Generate the output tokens for the task function, which includes:
    // - A static task descriptor with the unique ID and function name
    // - Registering the watchdog runner reference as a bound watchdog with the descriptor and timeout
    // - The user body of the function, which now has access to the bound watchdog for feeding
    let expanded = quote! {

        #[embassy_executor::task]
        #vis #sig {
            // Unique descriptor: contains the (no linker section)
            static #desc_ident: ::embassy_task_watchdog::TaskDesc = ::embassy_task_watchdog::TaskDesc {
                name: ::core::stringify!(#fn_ident),
            };
            // Convert runner ref into a per-task bound watchdog
            let #wd_ident = #wd_ident.register_desc(&#desc_ident, #desc_id, #max_expr).await;
            {
                // User body now sees #wd_ident: BoundWatchdog
                #block
            }
        }
    };

    expanded.into()
}
