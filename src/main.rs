#[macro_use]
extern crate rocket;

use itertools::Itertools;
use rocket::{
  response::status,
  serde::{json::Json, Deserialize, Serialize},
  Config,
};
use rocket_okapi::{
  okapi::{schemars, schemars::JsonSchema},
  openapi, openapi_get_routes, rapidoc,
  settings::UrlObject,
  swagger_ui,
};

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct ExecuteRequest {
  language: String,
  version: String,
  code: String,
  test: Test,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct Test {
  input: String,
  output: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct RunJson {
  stdout: String,
  stderr: String,
  code: Option<i64>,
  signal: Option<String>,
  output: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct ExecuteJson {
  run: RunJson,
  language: String,
  version: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct RuntimeReturnJson {
  language: String,
  version: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct RuntimeJson {
  language: String,
  version: String,
  aliases: Vec<String>,
  runtime: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct FileJson {
  name: Option<String>,
  content: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct PostExecuteJson {
  language: String,
  version: String,
  files: Vec<FileJson>,
  stdin: Option<String>,
  args: Option<Vec<String>>,
  compile_timeout: Option<i64>,
  run_timeout: Option<i64>,
  compile_memory_limit: Option<i64>,
  run_memory_limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct ExecuteResult {
  stdout: String,
  stderr: Option<String>,
  passed: bool,
}

#[openapi]
#[post("/submit", data = "<data>")]
async fn submit(data: Json<ExecuteRequest>) -> Result<Json<ExecuteResult>, status::NotFound<String>> {
  let execute_api = "http://localhost:2000/api/v2/execute";

  let execute_json = PostExecuteJson {
    language: data.language.to_owned(),
    version: data.version.to_owned(),
    args: None,
    compile_memory_limit: None,
    compile_timeout: None,
    run_memory_limit: None,
    run_timeout: None,
    stdin: Some(data.test.input.to_owned()),
    files: vec![FileJson {
      name: None,
      content: data.code.to_owned(),
    }],
  };

  let res = reqwest::Client::new()
    .post(execute_api)
    .json(&execute_json)
    .send()
    .await
    .map_err(|e| status::NotFound(e.to_string()))?
    .json::<ExecuteJson>()
    .await
    .map_err(|e| status::NotFound(e.to_string()))?;

  let stdout = if res.run.stdout.ends_with('\n') {
    // chop off \n if it ends with it
    res.run.stdout[0..res.run.stdout.len() - 1].to_string()
  } else {
    res.run.stdout.to_owned()
  };

  Ok(Json(ExecuteResult {
    stdout: res.run.stdout.to_owned(),
    stderr: if res.run.stderr.is_empty() {
      None
    } else {
      Some(res.run.stderr)
    },
    passed: stdout == data.test.output,
  }))
}

#[openapi]
#[get("/runtimes")]
async fn runtimes() -> Result<Json<Vec<RuntimeReturnJson>>, status::NotFound<String>> {
  let runtimes_api = "http://localhost:2000/api/v2/runtimes";

  let res = reqwest::get(runtimes_api)
    .await
    .map_err(|e| status::NotFound(e.to_string()))?
    .json::<Vec<RuntimeJson>>()
    .await
    .map_err(|e| status::NotFound(e.to_string()))?
    .iter()
    .map(|r| RuntimeReturnJson {
      language: r.language.to_owned(),
      version: r.version.to_owned(),
    })
    .collect_vec();

  Ok(Json(res))
}

#[launch]
fn rocket() -> _ {
  rocket::custom(Config {
    port: 5000,
    ..Config::default()
  })
  .mount("/", openapi_get_routes![runtimes, submit])
  .mount(
    "/swagger-ui/",
    swagger_ui::make_swagger_ui(&swagger_ui::SwaggerUIConfig {
      url: "../openapi.json".to_owned(),
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
