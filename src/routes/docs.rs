use actix_web::{get, HttpRequest, HttpResponse};

use crate::Result;

const OPENAPI_SCHEMA: &str = include_str!("../../openapi.yml");
static DOCS_HTML: &str = r##"\
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <title>website-screenshot docs</title>
        <link rel="stylesheet" type="text/css" href="swagger-ui.css" />
        <link rel="stylesheet" type="text/css" href="style.css" />
    <body>
        <div id="swagger-ui"></div>
        <script src="swagger-ui-bundle.js" charset="UTF-8"></script>
        <script src="swagger-ui-standalone-preset.js" charset="UTF-8"></script>
        <script src="swagger-initializer.js" charset="UTF-8"></script>
    </body>
</html>
"##;

#[get("/openapi.yml")]
pub async fn schema(req: HttpRequest) -> Result<HttpResponse> {
    let uri = req.uri();
    let host = req
        .headers()
        .get("Host")
        .expect("Failed getting host")
        .to_str()
        .expect("Failed converting host to str");

    let origin = format!("{}://{}", uri.scheme_str().unwrap_or("http"), host);

    let schema = OPENAPI_SCHEMA
        .replace("{{version}}", env!("CARGO_PKG_VERSION"))
        .replace("{{url}}", &origin);

    Ok(HttpResponse::Ok().content_type("text/yaml").body(schema))
}

#[get("/docs")]
pub async fn docs(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(DOCS_HTML)
}
