#[macro_use]
extern crate rocket;

use rocket::{response::status, serde::json::Json, Config};
use rocket_okapi::{openapi, openapi_get_routes, rapidoc, settings::UrlObject, swagger_ui};

const PORT: u16 = 5000;

#[openapi]
#[post("/execute", data = "<data>")]
async fn execute(data: Json<runner::ExecuteCodeRequest>) -> Result<Json<runner::Execution>, status::NotFound<String>> {
  runner::piston_execute(data.clone())
    .await
    .map(Json)
    .map_err(status::NotFound)
}

#[openapi]
#[post("/run_tests", data = "<data>")]
async fn run_tests(data: Json<runner::RunTests>) -> Result<Json<runner::ExecuteWithTests>, status::NotFound<String>> {
  runner::execute_with_tests(data.clone())
    .await
    .map(Json)
    .map_err(status::NotFound)
}

#[openapi]
#[get("/runtimes")]
async fn runtimes() -> Result<Json<Vec<runner::Runtime>>, status::NotFound<String>> {
  runner::piston_runtimes().await.map(Json).map_err(status::NotFound)
}

#[launch]
fn rocket() -> _ {
  rocket::custom(Config {
    port: PORT,
    ..Config::default()
  })
  .mount("/", openapi_get_routes![runtimes, execute, run_tests])
  .mount(
    "/swagger-ui/",
    swagger_ui::make_swagger_ui(&swagger_ui::SwaggerUIConfig {
      url: "../openapi.json".to_string(),
      ..Default::default()
    }),
  )
  .mount(
    "/rapidoc/",
    rapidoc::make_rapidoc(&rapidoc::RapiDocConfig {
      general: rapidoc::GeneralConfig {
        spec_urls: vec![UrlObject::new("General", "../openapi.json")],
        ..Default::default()
      },
      hide_show: rapidoc::HideShowConfig {
        allow_spec_url_load: false,
        allow_spec_file_load: false,
        ..Default::default()
      },
      ..Default::default()
    }),
  )
}
