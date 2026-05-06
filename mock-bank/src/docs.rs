use utoipa::OpenApi;
use crate::models::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::authorize,
        crate::routes::capture,
        crate::routes::void,
        crate::routes::refund,
    ),
    components(
        schemas(
            AuthorizeRequest, AuthorizeResponse,
            CaptureRequest, CaptureResponse,
            VoidRequest, VoidResponse,
            RefundRequest, RefundResponse,
            BankErrorResponse,
            common::CardDetails
        )
    ),
    tags(
        (name = "mock-bank", description = "Mock Bank API")
    )
)]
pub struct ApiDoc;
