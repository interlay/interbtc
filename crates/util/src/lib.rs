use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, Result};

// todo: replace by the version in frame_support when a version of substrate is released that includes this commit:
// https://github.com/paritytech/substrate/commit/c2356994cda16332b71ab4abdaebd4f2d08bfdfa#diff-329ac4bf9a3297b48ea4b309c60423a2df210c8e1e5bf57aea28b14c61d12211R32
#[proc_macro_attribute]
pub fn transactional(attr: TokenStream, input: TokenStream) -> TokenStream {
    _transactional(attr, input).unwrap_or_else(|e| e.to_compile_error().into())
}

fn _transactional(_attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = syn::parse(input)?;

    let output = quote! {
        #(#attrs)*
        #vis #sig {
            use frame_support::storage::{with_transaction, TransactionOutcome};
            with_transaction(|| {
                let r = (|| { #block })();
                if r.is_ok() {
                    TransactionOutcome::Commit(r)
                } else {
                    TransactionOutcome::Rollback(r)
                }
            })
        }
    };

    Ok(output.into())
}
