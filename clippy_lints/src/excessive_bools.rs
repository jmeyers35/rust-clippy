use crate::utils::{attr_by_name, in_macro, match_path_ast, span_lint_and_help};
use rustc_ast::ast::{AssocItemKind, Extern, FnSig, Item, ItemKind, Ty, TyKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;

use std::convert::TryInto;

declare_clippy_lint! {
    /// **What it does:** Checks for excessive
    /// use of bools in structs.
    ///
    /// **Why is this bad?** Excessive bools in a struct
    /// is often a sign that it's used as a state machine,
    /// which is much better implemented as an enum.
    /// If it's not the case, excessive bools usually benefit
    /// from refactoring into two-variant enums for better
    /// readability and API.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// Bad:
    /// ```rust
    /// struct S {
    ///     is_pending: bool,
    ///     is_processing: bool,
    ///     is_finished: bool,
    /// }
    /// ```
    ///
    /// Good:
    /// ```rust
    /// enum S {
    ///     Pending,
    ///     Processing,
    ///     Finished,
    /// }
    /// ```
    pub STRUCT_EXCESSIVE_BOOLS,
    pedantic,
    "using too many bools in a struct"
}

declare_clippy_lint! {
    /// **What it does:** Checks for excessive use of
    /// bools in function definitions.
    ///
    /// **Why is this bad?** Calls to such functions
    /// are confusing and error prone, because it's
    /// hard to remember argument order and you have
    /// no type system support to back you up. Using
    /// two-variant enums instead of bools often makes
    /// API easier to use.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// Bad:
    /// ```rust,ignore
    /// fn f(is_round: bool, is_hot: bool) { ... }
    /// ```
    ///
    /// Good:
    /// ```rust,ignore
    /// enum Shape {
    ///     Round,
    ///     Spiky,
    /// }
    ///
    /// enum Temperature {
    ///     Hot,
    ///     IceCold,
    /// }
    ///
    /// fn f(shape: Shape, temperature: Temperature) { ... }
    /// ```
    pub FN_PARAMS_EXCESSIVE_BOOLS,
    pedantic,
    "using too many bools in function parameters"
}

pub struct ExcessiveBools {
    max_struct_bools: u64,
    max_fn_params_bools: u64,
}

impl ExcessiveBools {
    #[must_use]
    pub fn new(max_struct_bools: u64, max_fn_params_bools: u64) -> Self {
        Self {
            max_struct_bools,
            max_fn_params_bools,
        }
    }

    fn check_fn_sig(&self, cx: &EarlyContext<'_>, fn_sig: &FnSig, span: Span) {
        match fn_sig.header.ext {
            Extern::Implicit | Extern::Explicit(_) => return,
            Extern::None => (),
        }

        let fn_sig_bools = fn_sig
            .decl
            .inputs
            .iter()
            .filter(|param| is_bool_ty(&param.ty))
            .count()
            .try_into()
            .unwrap();
        if self.max_fn_params_bools < fn_sig_bools {
            span_lint_and_help(
                cx,
                FN_PARAMS_EXCESSIVE_BOOLS,
                span,
                &format!("more than {} bools in function parameters", self.max_fn_params_bools),
                "consider refactoring bools into two-variant enums",
            );
        }
    }
}

impl_lint_pass!(ExcessiveBools => [STRUCT_EXCESSIVE_BOOLS, FN_PARAMS_EXCESSIVE_BOOLS]);

fn is_bool_ty(ty: &Ty) -> bool {
    if let TyKind::Path(None, path) = &ty.kind {
        return match_path_ast(path, &["bool"]);
    }
    false
}

impl EarlyLintPass for ExcessiveBools {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if in_macro(item.span) {
            return;
        }
        match &item.kind {
            ItemKind::Struct(variant_data, _) => {
                if attr_by_name(&item.attrs, "repr").is_some() {
                    return;
                }

                let struct_bools = variant_data
                    .fields()
                    .iter()
                    .filter(|field| is_bool_ty(&field.ty))
                    .count()
                    .try_into()
                    .unwrap();
                if self.max_struct_bools < struct_bools {
                    span_lint_and_help(
                        cx,
                        STRUCT_EXCESSIVE_BOOLS,
                        item.span,
                        &format!("more than {} bools in a struct", self.max_struct_bools),
                        "consider using a state machine or refactoring bools into two-variant enums",
                    );
                }
            },
            ItemKind::Impl {
                of_trait: None, items, ..
            }
            | ItemKind::Trait(_, _, _, _, items) => {
                for item in items {
                    if let AssocItemKind::Fn(_, fn_sig, _, _) = &item.kind {
                        self.check_fn_sig(cx, fn_sig, item.span);
                    }
                }
            },
            ItemKind::Fn(_, fn_sig, _, _) => self.check_fn_sig(cx, fn_sig, item.span),
            _ => (),
        }
    }
}
