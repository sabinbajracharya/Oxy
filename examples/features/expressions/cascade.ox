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
    assert_eq(api.port, 80);
}

#[test]
fn test_apply_trailing_closure_with_parens() {
    val api = ApiService::new().apply() {
        it.port = 81;
    };
    assert_eq(api.port, 81);
}

#[test]
fn test_try_apply_returns_result() {
    val api_result = ApiService::new().try_apply {
        it.port = 90;
        Ok(())
    };

    match api_result {
        Ok(api) => assert_eq(api.port, 90),
        Err(_) => assert(false),
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
        Ok(_) => assert(false),
        Err(_) => assert(true),
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
        Ok(api) => assert_eq(api.port, 92),
        Err(_) => assert(false),
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
