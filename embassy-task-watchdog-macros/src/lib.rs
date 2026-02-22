use core::sync::atomic::{AtomicU32, Ordering};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
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
/// The [`task`] decorator macro is used in the place of [`embassy_executor::task`] to create an async task that can be monitored by the task-watchdog.  It is used like this, for a task that feeds the watchdog every 1000ms and is considered stalled if it goes more than 2000ms without feeding:
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
/// The first argument to the task must be a static reference to the [`embassy_task_watchdog::TaskWatchdog`] for the task to register itself with.  The macro will convert this into a per-task bound watchdog that the user can feed to indicate the task is still alive.  The `timeout` argument specifies how long the watchdog should wait for a feed before considering the task to be stalled.  This is required to be able to detect stalls, and should be set to a value that is longer than the longest expected time between feeds in the task.
pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TaskArgs);
    let f = parse_macro_input!(item as ItemFn);

    if f.sig.asyncness.is_none() {
        return syn::Error::new(
            f.sig.span(),
            "#[embassy_task_watchdog::task] must be on async fn",
        )
        .to_compile_error()
        .into();
    }

    let wd_ident = match first_param_ident(&f) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error().into(),
    };

    let desc_id = TASK_ID.fetch_add(1, Ordering::SeqCst);
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

    let vis = &f.vis;
    let sig = &f.sig;
    let fn_ident = &f.sig.ident;
    let block = &f.block;

    let desc_ident = format_ident!(
        "__EMBASSY_TASK_WATCHDOG_DESC_{}",
        fn_ident.to_string().to_uppercase()
    );

    let max_expr = args.timeout;

    let expanded = quote! {

        #[embassy_executor::task]
        #vis #sig {
            // Unique descriptor: address acts as identity (no linker section)
            static #desc_ident: ::embassy_task_watchdog::TaskDesc = ::embassy_task_watchdog::TaskDesc {
                name: ::core::stringify!(#fn_ident),
                id: #desc_id,
            };
            // Convert runner ref into a per-task bound watchdog
            let #wd_ident = #wd_ident.register_desc(&#desc_ident, #max_expr).await;
            {
                // User body now sees #wd_ident: BoundWatchdog
                #block
            }
            #[allow(unreachable_code)]
            #wd_ident.deregister().await;
        }
    };

    expanded.into()
}
