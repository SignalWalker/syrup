use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    parse::{discouraged::Speculative, Parse},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Ident, LitBool, Token,
};

pub(crate) struct AttrFlag {
    pub(crate) ident: Ident,
    pub(crate) eq: Option<Token![=]>,
    pub(crate) value: LitBool,
}

impl Parse for AttrFlag {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        let (eq, value) = {
            if let Ok(eq) = input.parse::<Token![=]>() {
                (Some(eq), input.parse()?)
            } else {
                (None, LitBool::new(true, ident.span()))
            }
        };
        Ok(Self { ident, eq, value })
    }
}

impl ToTokens for AttrFlag {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        if let Some(eq) = &self.eq {
            eq.to_tokens(tokens);
            self.value.to_tokens(tokens);
        }
    }
}

impl From<AttrFlag> for LitBool {
    fn from(value: AttrFlag) -> Self {
        value.value
    }
}

pub(crate) struct AttrOptionSet {
    pub(crate) ident: Ident,
    pub(crate) options: HashMap<String, AttrOption>,
}

impl Parse for AttrOptionSet {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            options: {
                let inner;
                let _ = syn::parenthesized!(inner in input);
                let mut opts = HashMap::new();
                for opt in Punctuated::<AttrOption, Token![,]>::parse_terminated(&inner)? {
                    opts.insert(opt.ident().to_string(), opt);
                }
                opts
            },
        })
    }
}

impl AttrOptionSet {
    pub(crate) fn remove_implicit<T: TryFrom<AttrOption, Error = syn::Error> + From<Span>>(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        let span = self.ident.span();
        self.options
            .remove(key)
            .map(TryFrom::try_from)
            .unwrap_or_else(|| Ok(T::from(span)))
    }

    pub(crate) fn remove<T: TryFrom<AttrOption, Error = syn::Error>>(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        match self.options.remove(key) {
            Some(v) => v.try_into(),
            None => error!(&self.ident => "missing {key}"),
        }
    }

    pub(crate) fn try_remove_or<T: TryFrom<AttrOption, Error = syn::Error>>(
        &mut self,
        key: &str,
        or: impl FnOnce() -> syn::Result<T>,
    ) -> syn::Result<T> {
        self.options
            .remove(key)
            .map(TryFrom::try_from)
            .unwrap_or_else(or)
    }

    pub(crate) fn remove_flag_or(&mut self, key: &str, default: bool) -> syn::Result<LitBool> {
        let span = self.ident.span();
        self.try_remove_or(key, || Ok(LitBool::new(default, span)))
    }

    pub(crate) fn remove_set<T: TryFrom<AttrOptionSet, Error = syn::Error>>(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        match self.options.remove(key) {
            Some(AttrOption::Set(set)) => set.try_into(),
            Some(AttrOption::Flag(flag)) => error!(flag => "expected arguments"),
            None => error!(&self.ident => "missing {key}"),
        }
    }

    pub(crate) fn remove_implicit_set<
        T: TryFrom<AttrOptionSet, Error = syn::Error> + From<Span>,
    >(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        match self.options.remove(key) {
            Some(AttrOption::Set(set)) => set.try_into(),
            Some(AttrOption::Flag(flag)) => error!(flag => "expected arguments"),
            None => Ok(T::from(self.ident.span())),
        }
    }

    pub(crate) fn into_unrecognized_err(self) -> syn::Result<()> {
        for opt in self.options.values() {
            error!(opt => "unrecognized attribute option");
        }
        Ok(())
    }

    pub(crate) fn process(attr: &Attribute) -> syn::Result<Self> {
        let ident = attr.path().require_ident()?.clone();
        let mut options = HashMap::new();
        for opt in attr.parse_args_with(Punctuated::<AttrOption, Token![,]>::parse_terminated)? {
            options.insert(opt.ident().to_string(), opt);
        }
        Ok(Self { ident, options })
    }
}

impl ToTokens for AttrOptionSet {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        // FIX :: parentheses...?
        tokens.append_separated(self.options.values(), Token![,](self.ident.span()));
    }
}

pub(crate) enum AttrOption {
    Flag(AttrFlag),
    Set(AttrOptionSet),
}

impl TryFrom<AttrOption> for LitBool {
    type Error = syn::Error;

    fn try_from(value: AttrOption) -> Result<Self, Self::Error> {
        match value {
            AttrOption::Flag(flag) => Ok(flag.value),
            AttrOption::Set(set) => error!(set => "unexpected attribute option arguments"),
        }
    }
}

impl AttrOption {
    fn ident(&self) -> &Ident {
        match self {
            AttrOption::Flag(flag) => &flag.ident,
            AttrOption::Set(set) => &set.ident,
        }
    }
}

impl ToTokens for AttrOption {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrOption::Flag(flag) => flag.to_tokens(tokens),
            AttrOption::Set(set) => set.to_tokens(tokens),
        }
    }
}

impl Parse for AttrOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // TODO :: is there a better way to do this...?
        let fork = input.fork();
        if let Ok(set) = fork.parse() {
            input.advance_to(&fork);
            Ok(Self::Set(set))
        } else {
            input.parse().map(Self::Flag)
        }
    }
}
