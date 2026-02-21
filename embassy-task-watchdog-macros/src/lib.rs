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
    max_duration: Expr, // must evaluate to embassy_time::Duration
}

impl Parse for TaskArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut max_duration: Option<Expr> = None;
        let args: Punctuated<MetaNameValue, Token![,]> =
            input.parse_terminated(MetaNameValue::parse, Token![,])?;

        for nv in args {
            let key = nv.path.get_ident().map(|i| i.to_string());
            match key.as_deref() {
                Some("max_duration") => max_duration = Some(nv.value),
                Some(other) => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        format!("unknown argument `{other}` (supported: max_duration)"),
                    ));
                }
                None => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        "expected identifier key (supported: max_duration)",
                    ));
                }
            }
        }

        Ok(Self {
            max_duration: max_duration.ok_or_else(|| {
                syn::Error::new(input.span(), "missing required: max_duration = <expr>")
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

    let vis = &f.vis;
    let sig = &f.sig;
    let fn_ident = &f.sig.ident;
    let block = &f.block;

    let desc_ident = format_ident!(
        "__EMBASSY_TASK_WATCHDOG_DESC_{}",
        fn_ident.to_string().to_uppercase()
    );
    let maxfn_ident = format_ident!("__embassy_task_watchdog_max_{}", fn_ident);

    let max_expr = args.max_duration;

    let expanded = quote! {
        #[inline(always)]
        const fn #maxfn_ident() -> ::embassy_time::Duration {
            #max_expr
        }

        // Unique descriptor: address acts as identity (no linker section)
        static #desc_ident: ::embassy_task_watchdog::TaskDesc = ::embassy_task_watchdog::TaskDesc {
            name: ::core::stringify!(#fn_ident),
            max_duration: #maxfn_ident,
        };

        #[embassy_executor::task]
        #vis #sig {
            // Convert runner ref into a per-task bound watchdog
            let #wd_ident = #wd_ident.register_desc(&#desc_ident).await;

            // User body now sees #wd_ident: BoundWatchdog
            #block
        }
    };

    expanded.into()
}
