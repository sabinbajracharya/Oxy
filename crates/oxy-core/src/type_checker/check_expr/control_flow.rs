//! Type inference for branching and block expressions: `if`, `if let`,
//! `match` (with exhaustiveness), blocks, and `return`.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_if(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        self.infer_expr(condition)?;
        let then_ty = self.block_tail_type(then_block)?;
        let result = if let Some(else_expr) = else_block {
            let else_ty = self.infer_expr(else_expr)?;
            self.unify_branch_types(&then_ty, &else_ty, "if", *span)?
        } else {
            then_ty
        };
        Ok(result)
    }

    pub(super) fn infer_if_let(
        &mut self,
        pattern: &Pattern,
        inner: &Expr,
        guard: &Option<Box<Expr>>,
        then_block: &Block,
        else_block: &Option<Box<Expr>>,
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let _ = self.infer_expr(inner)?;
        let saved = self.env.clone();
        self.env = TypeEnv::child(&saved);
        self.bind_pattern(pattern, false);
        if let Some(g) = guard {
            let _ = self.infer_expr(g)?;
        }
        let then_ty = self.block_tail_type(then_block)?;
        self.env = saved;
        let result = if let Some(else_expr) = else_block {
            let else_ty = self.infer_expr(else_expr)?;
            self.unify_branch_types(&then_ty, &else_ty, "if let", *span)?
        } else {
            then_ty
        };
        Ok(result)
    }

    pub(super) fn infer_match(
        &mut self,
        matched: &Expr,
        arms: &[MatchArm],
        span: &Span,
    ) -> Result<TypeInfo, FerriError> {
        let matched_ty = self.infer_expr(matched)?;
        if !self.match_is_exhaustive(&matched_ty, arms) {
            return Err(FerriError::TypeError {
                message: "non-exhaustive match: add a `_ =>` arm or cover all cases".to_string(),
                line: span.line,
                column: span.column,
            });
        }
        let mut arm_types: Vec<TypeInfo> = Vec::with_capacity(arms.len());
        for arm in arms {
            let arm_env = TypeEnv::child(&self.env);
            let saved = self.env.clone();
            self.env = arm_env;
            self.bind_pattern(&arm.pattern, false);
            if let Some(g) = &arm.guard {
                let _ = self.infer_expr(g)?;
            }
            let arm_ty = self.infer_expr(&arm.body)?;
            self.env = saved;
            arm_types.push(arm_ty);
        }
        // Pick the first non-Unit/non-Unknown arm as the leader,
        // then require all other producing-arms to unify with it.
        let mut leader: TypeInfo = TypeInfo::Unit;
        for t in &arm_types {
            if *t == TypeInfo::Unknown || *t == TypeInfo::Unit {
                continue;
            }
            if leader == TypeInfo::Unit {
                leader = t.clone();
                continue;
            }
            leader = self.unify_branch_types(&leader, t, "match", *span)?;
        }
        Ok(leader)
    }

    pub(super) fn infer_block(&mut self, block: &Block) -> Result<TypeInfo, FerriError> {
        let mut last_ty = TypeInfo::Unit;
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.check_stmt(stmt, &TypeInfo::Unknown)?;
            if is_last {
                if let Stmt::Expr {
                    expr,
                    has_semicolon,
                } = stmt
                {
                    if !has_semicolon {
                        last_ty = self.infer_expr(expr)?;
                    }
                }
            }
        }
        Ok(last_ty)
    }

    pub(super) fn infer_return(
        &mut self,
        value: &Option<Box<Expr>>,
    ) -> Result<TypeInfo, FerriError> {
        if let Some(expr) = value {
            let _ = self.infer_expr(expr)?;
        }
        Ok(TypeInfo::Unknown) // diverging expression
    }

    /// Decide whether a `match` covers every possible value of `matched_ty`.
    /// Conservative: only reports non-exhaustive when confident (a closed
    /// domain — scalar, bool, or known enum — with no catch-all). Unknown or
    /// open types are assumed exhaustive to avoid false positives.
    fn match_is_exhaustive(&self, matched_ty: &TypeInfo, arms: &[MatchArm]) -> bool {
        // A guardless irrefutable arm covers everything that remains.
        if arms
            .iter()
            .any(|a| a.guard.is_none() && Self::pattern_is_irrefutable(&a.pattern))
        {
            return true;
        }
        // Bool: exhaustive iff both `true` and `false` are matched literally.
        if *matched_ty == TypeInfo::Bool {
            let (mut seen_true, mut seen_false) = (false, false);
            for arm in arms {
                if arm.guard.is_some() {
                    continue;
                }
                Self::collect_bool_literals(&arm.pattern, &mut seen_true, &mut seen_false);
            }
            return seen_true && seen_false;
        }
        // Known enum: exhaustive iff every variant appears in a guardless arm.
        if let TypeInfo::UserStruct { name, .. } = matched_ty {
            if let Some(variants) = self.enum_variants.get(name) {
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    if arm.guard.is_some() {
                        continue;
                    }
                    Self::collect_covered_variants(&arm.pattern, &mut covered);
                }
                return variants.iter().all(|v| covered.contains(v.as_str()));
            }
        }
        // Scalar domains (int/byte/float/string/char) are effectively
        // unbounded — without a catch-all they cannot be exhaustive.
        if matched_ty.is_integer()
            || matched_ty.is_float()
            || *matched_ty == TypeInfo::String
            || *matched_ty == TypeInfo::Char
        {
            return false;
        }
        // Open / unknown types: assume exhaustive (conservative).
        true
    }

    /// A pattern that matches any value of its type (binds or discards), so an
    /// arm using it is a catch-all.
    fn pattern_is_irrefutable(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard(_) | Pattern::Ident(_, _) | Pattern::Rest(_) => true,
            Pattern::Or(pats, _) => pats.iter().any(Self::pattern_is_irrefutable),
            Pattern::Tuple(pats, _) => pats.iter().all(Self::pattern_is_irrefutable),
            _ => false,
        }
    }

    fn collect_bool_literals(pattern: &Pattern, seen_true: &mut bool, seen_false: &mut bool) {
        match pattern {
            Pattern::Literal(Expr::BoolLiteral(true, _)) => *seen_true = true,
            Pattern::Literal(Expr::BoolLiteral(false, _)) => *seen_false = true,
            Pattern::Or(pats, _) => {
                for p in pats {
                    Self::collect_bool_literals(p, seen_true, seen_false);
                }
            }
            _ => {}
        }
    }

    fn collect_covered_variants<'a>(
        pattern: &'a Pattern,
        covered: &mut std::collections::HashSet<&'a str>,
    ) {
        match pattern {
            Pattern::EnumVariant { variant, .. } => {
                covered.insert(variant.as_str());
            }
            Pattern::Or(pats, _) => {
                for p in pats {
                    Self::collect_covered_variants(p, covered);
                }
            }
            _ => {}
        }
    }
}
