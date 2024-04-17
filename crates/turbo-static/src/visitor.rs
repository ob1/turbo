//! A visitor that traverses the AST and collects all functions or methods that
//! are annotated with `#[turbo_tasks::function]`.

use std::{collections::VecDeque, ops::Add};

use syn::{spanned::Spanned, visit::Visit, Expr, Meta};

pub struct TaskVisitor {
    /// the list of results as pairs of an identifier and its tags
    pub results: Vec<(syn::Ident, Vec<String>)>,
}

impl TaskVisitor {
    pub fn new() -> Self {
        Self {
            results: Default::default(),
        }
    }
}

impl Visit<'_> for TaskVisitor {
    #[tracing::instrument(skip_all)]
    fn visit_item_fn(&mut self, i: &syn::ItemFn) {
        if let Some(tags) = extract_tags(i.attrs.iter()) {
            tracing::trace!("L{}: {}", i.sig.ident.span().start().line, i.sig.ident,);
            self.results.push((i.sig.ident.clone(), tags));
        }
    }

    #[tracing::instrument(skip_all)]
    fn visit_impl_item_fn(&mut self, i: &syn::ImplItemFn) {
        if let Some(tags) = extract_tags(i.attrs.iter()) {
            tracing::trace!("L{}: {}", i.sig.ident.span().start().line, i.sig.ident,);
            self.results.push((i.sig.ident.clone(), tags));
        }
    }
}

fn extract_tags<'a>(mut meta: impl Iterator<Item = &'a syn::Attribute>) -> Option<Vec<String>> {
    meta.find_map(|a| match &a.meta {
        // path has two segments, turbo_tasks and function
        Meta::Path(path) if path.segments.len() == 2 => {
            let first = &path.segments[0];
            let second = &path.segments[1];
            (first.ident == "turbo_tasks" && second.ident == "function").then(std::vec::Vec::new)
        }
        Meta::List(list) if list.path.segments.len() == 2 => {
            let first = &list.path.segments[0];
            let second = &list.path.segments[1];
            if first.ident != "turbo_tasks" || second.ident != "function" {
                return None;
            }

            // collect ident tokens as args
            let tags: Vec<_> = list
                .tokens
                .clone()
                .into_iter()
                .filter_map(|t| {
                    if let proc_macro2::TokenTree::Ident(ident) = t {
                        Some(ident.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            Some(tags)
        }
        _ => {
            tracing::trace!("skipping unknown annotation");
            None
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum CallingStyle {
    Once,
    #[allow(dead_code)]
    ZeroOrOnce,
    #[allow(dead_code)]
    ZeroOrMore,
    #[allow(dead_code)]
    OneOrMore,
}

impl CallingStyle {
    fn bitset(self) -> u8 {
        match self {
            CallingStyle::Once => 0b0010,
            CallingStyle::ZeroOrOnce => 0b011,
            CallingStyle::ZeroOrMore => 0b0111,
            CallingStyle::OneOrMore => 0b0110,
        }
    }
}

impl Add for CallingStyle {
    type Output = Self;

    /// Add two calling styles together to determine the calling style of the
    /// target function within the source function.
    ///
    /// Consider it as a bitset over properties.
    /// - 0b0001: Zero
    /// - 0b0010: Once
    /// - 0b0100: More Than Once
    ///
    /// Note that zero is not a valid calling style.
    fn add(self, rhs: Self) -> Self {
        let left = self.bitset();
        let right = rhs.bitset();
        match left | right {
            0b0010 => CallingStyle::Once,
            0b011 => CallingStyle::ZeroOrOnce,
            0b0111 => CallingStyle::ZeroOrMore,
            0b0110 => CallingStyle::OneOrMore,
            _ => unreachable!(),
        }
    }
}

pub struct CallingStyleVisitor {
    pub reference: crate::IdentifierReference,
    state: VecDeque<CallingStyleVisitorState>,
}

impl CallingStyleVisitor {
    /// Create a new visitor that will traverse the AST and determine the
    /// calling style of the target function within the source function.
    pub fn new(reference: crate::IdentifierReference) -> Self {
        Self {
            reference,
            state: Default::default(),
        }
    }

    pub fn result(self) -> Option<CallingStyle> {
        self.state
            .into_iter()
            .map(|b| match b {
                CallingStyleVisitorState::Block => CallingStyle::Once,
                CallingStyleVisitorState::Loop => CallingStyle::ZeroOrMore,
                CallingStyleVisitorState::If => CallingStyle::ZeroOrOnce,
                CallingStyleVisitorState::Closure => CallingStyle::ZeroOrMore,
            })
            .reduce(|a, b| a + b)
    }
}

#[derive(Debug, Clone, Copy)]
enum CallingStyleVisitorState {
    Block,
    Loop,
    If,
    Closure,
}

impl Visit<'_> for CallingStyleVisitor {
    fn visit_item_fn(&mut self, i: &'_ syn::ItemFn) {
        if self.reference.identifier.equals_ident(&i.sig.ident, true) {
            self.state.push_back(CallingStyleVisitorState::Block);
            syn::visit::visit_item_fn(self, i);
            self.state.pop_back();
        }
    }

    fn visit_impl_item_fn(&mut self, i: &'_ syn::ImplItemFn) {
        if self.reference.identifier.equals_ident(&i.sig.ident, true) {
            self.state.push_back(CallingStyleVisitorState::Block);
            syn::visit::visit_impl_item_fn(self, i);
            self.state.pop_back();
        }
    }

    fn visit_expr_loop(&mut self, i: &'_ syn::ExprLoop) {
        self.state.push_back(CallingStyleVisitorState::Loop);
        syn::visit::visit_expr_loop(self, i);
        self.state.pop_back();
    }

    fn visit_expr_for_loop(&mut self, i: &'_ syn::ExprForLoop) {
        self.state.push_back(CallingStyleVisitorState::Loop);
        syn::visit::visit_expr_for_loop(self, i);
        self.state.pop_back();
    }

    fn visit_expr_if(&mut self, i: &'_ syn::ExprIf) {
        self.state.push_back(CallingStyleVisitorState::If);
        syn::visit::visit_expr_if(self, i);
        self.state.pop_back();
    }

    fn visit_expr_closure(&mut self, i: &'_ syn::ExprClosure) {
        self.state.push_back(CallingStyleVisitorState::Closure);
        syn::visit::visit_expr_closure(self, i);
        self.state.pop_back();
    }

    fn visit_expr_call(&mut self, i: &'_ syn::ExprCall) {
        match i.func.as_ref() {
            Expr::Path(p) => {
                println!("{:?} - {:?}", p.span(), self.reference.references)
            }
            rest => {
                tracing::info!("visiting call: {:?}", rest);
            }
        }
    }
}
