use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Lit, Meta, NestedMeta, Result, Token};

pub struct Args {
    pub vars: Vec<Ident>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let vars = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        Ok(Args {
            vars: vars.into_iter().collect(),
        })
    }
}

pub struct Command;

impl Command {
    pub fn get_varname(ident: &Ident) -> Ident {
        format_ident!("static_command_for_{}", ident.to_string())
    }
}

pub fn get_string_value(meta: &NestedMeta, key: &str) -> String {
    let expect_msg = format!("Expected argument `{} = \"...\"`", key);
    let argument_name_and_value = match meta {
        NestedMeta::Meta(Meta::NameValue(meta)) => meta,
        _ => panic!("{}", expect_msg),
    };
    assert_eq!(
        argument_name_and_value
            .path
            .segments
            .first()
            .expect(&expect_msg)
            .ident,
        key
    );
    match &argument_name_and_value.lit {
        Lit::Str(lit) => lit.value(),
        _ => panic!("{} argument must be a string", key),
    }
}
