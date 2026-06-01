// apply/try_apply closure ergonomics and strict return-boundary checks.

struct ApiService {
    port: Int,
}

enum AuthError {
    Denied,
}

impl ApiService {
    fn new() -> ApiService {
        ApiService { port: 0 }
    }

    fn auth_ok(self) -> Result<(), AuthError> {
        Ok(())
    }

    fn auth_fail(self) -> Result<(), AuthError> {
        Err(AuthError::Denied)
    }
}

#[test]
fn test_apply_trailing_closure_implicit_it() {
    val api = ApiService::new().apply {
        it.port = 80;
    };
    assert::eq(api.port, 80);
}

#[test]
fn test_apply_trailing_closure_with_parens() {
    val api = ApiService::new().apply() {
        it.port = 81;
    };
    assert::eq(api.port, 81);
}

#[test]
fn test_map_trailing_closure_implicit_it() {
    val xs = [1, 2, 3];
    val ys = xs.map { it + 1 };
    assert::eq(ys.len(), 3);
    assert::eq(ys[0], 2);
}

#[test]
fn test_try_apply_returns_result() {
    val api_result = ApiService::new().try_apply {
        it.port = 90;
        Ok(())
    };

    match api_result {
        Ok(api) => assert::eq(api.port, 90),
        Err(_) => assert::true(false),
    }
}

#[test]
fn test_try_apply_error_stays_local() {
    val api_result = ApiService::new().try_apply {
        it.port = 91;
        it.auth_fail()?;
        Ok(())
    };

    match api_result {
        Ok(_) => assert::true(false),
        Err(_) => assert::true(true),
    }
}

#[test]
fn test_try_apply_allows_explicit_return_err() {
    val api_result = ApiService::new().try_apply {
        if true {
            return Err(AuthError::Denied);
        }
        Ok(())
    };

    match api_result {
        Ok(_) => assert::true(false),
        Err(_) => assert::true(true),
    }
}

fn setup_api() -> Result<ApiService, AuthError> {
    val api = ApiService::new().try_apply {
        it.port = 92;
        it.auth_ok()?;
        Ok(())
    }?;
    Ok(api)
}

#[test]
fn test_try_apply_outer_question_bubbles() {
    match setup_api() {
        Ok(api) => assert::eq(api.port, 92),
        Err(_) => assert::true(false),
    }
}

#[compile_error]
fn test_apply_rejects_question_operator() {
    val _api = ApiService::new().apply {
        it.port = 80;
        it.auth_fail()?;
    };
}

#[compile_error]
fn test_apply_rejects_result_return() {
    val _api = ApiService::new().apply {
        it.port = 80;
        Err(AuthError::Denied)
    };
}

#[compile_error]
fn test_apply_rejects_explicit_return_err() {
    val _api = ApiService::new().apply {
        return Err(AuthError::Denied);
    };
}

#[compile_error]
fn test_apply_rejects_wrong_closure_param_type() {
    val _api = ApiService::new().apply(|it: Int| {
        val _ = it + 1;
    });
}

#[compile_error]
fn test_try_apply_rejects_wrong_closure_param_type() {
    val _api = ApiService::new().try_apply(|it: Int| {
        val _ = it + 1;
        Ok(())
    });
}

#[compile_error]
fn test_global_println_is_rejected() {
    println("legacy");
}

#[compile_error]
fn test_global_dbg_is_rejected() {
    dbg(123);
}

#[compile_error]
fn test_global_assert_is_rejected() {
    assert(true);
}
