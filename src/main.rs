#[macro_use]
extern crate rocket;

use cached::proc_macro::cached;
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

const PORT: u16 = 5000;

const COMPILE_MEMORY_LIMIT: i64 = 512 * 1024 * 1024;
const RUN_MEMORY_LIMIT: i64 = 512 * 1024 * 1024;

const EXECUTE_API: &str = "http://localhost:2000/api/v2/execute";
const RUNTIMES_API: &str = "http://localhost:2000/api/v2/runtimes";

#[derive(Serialize, Deserialize)]
struct PistonRun {
  stdout: String,
  stderr: String,
  code: Option<i64>,
  signal: Option<String>,
  output: String,
}

#[derive(Serialize, Deserialize)]
struct PistonExecution {
  run: PistonRun,
  language: String,
  version: String,
}

#[derive(Serialize, Deserialize)]
struct PistonFile {
  name: Option<String>,
  content: String,
}

#[derive(Serialize, Deserialize)]
struct PistonExecuteRequest {
  language: String,
  version: String,
  files: Vec<PistonFile>,
  stdin: Option<String>,
  args: Option<Vec<String>>,
  compile_timeout: Option<i64>,
  run_timeout: Option<i64>,
  compile_memory_limit: Option<i64>,
  run_memory_limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct PistonRuntime {
  language: String,
  version: String,
  aliases: Vec<String>,
  runtime: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
struct ExecuteCodeRequest {
  code: String,
  language: String,
  stdin: Option<String>,
  timeout: Option<i64>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct Execution {
  stdout: String,
  stderr: Option<String>,
  time_limit_exceeded: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct Runtime {
  language: String,
  version: String,
}

#[cached(time = 60, result = true)]
async fn piston_execute(data: ExecuteCodeRequest) -> Result<Execution, String> {
  if let Some(timeout) = data.timeout {
    if !(1..=3000).contains(&timeout) {
      return Err("Timeout must be between 1 and 3000 ms (inclusive)".to_string());
    }
  }

  let execute_json = PistonExecuteRequest {
    language: data.language.to_owned(),
    version: piston_runtimes()
      .await?
      .into_iter()
      .find(|runtime| runtime.language == data.language)
      .ok_or(format!("Couldn't find '{}' language", data.language))?
      .version,
    args: None,
    compile_memory_limit: Some(COMPILE_MEMORY_LIMIT),
    compile_timeout: None,
    run_memory_limit: Some(RUN_MEMORY_LIMIT),
    run_timeout: data.timeout,
    stdin: data.stdin,
    files: vec![PistonFile {
      name: None,
      content: data.code.to_owned(),
    }],
  };

  let res = reqwest::Client::new()
    .post(EXECUTE_API)
    .json(&execute_json)
    .send()
    .await
    .map_err(|e| e.to_string())?
    .json::<PistonExecution>()
    .await
    .map_err(|e| e.to_string())?;

  let stdout = if res.run.stdout.ends_with('\n') {
    // chop off \n if it ends with it
    res.run.stdout[0..res.run.stdout.len() - 1].to_string()
  } else {
    res.run.stdout.to_owned()
  };

  let time_limit_exceeded = res.run.signal.map(|s| s == "SIGKILL").unwrap_or(false);

  Ok(Execution {
    stdout,
    stderr: if res.run.stderr.is_empty() {
      None
    } else {
      Some(res.run.stderr)
    },
    time_limit_exceeded,
  })
}

#[openapi]
#[post("/execute", data = "<data>")]
async fn execute(data: Json<ExecuteCodeRequest>) -> Result<Json<Execution>, status::NotFound<String>> {
  piston_execute(data.clone()).await.map(Json).map_err(status::NotFound)
}

#[cached(time = 60, result = true)]
async fn piston_runtimes() -> Result<Vec<Runtime>, String> {
  Ok(
    reqwest::get(RUNTIMES_API)
      .await
      .map_err(|e| e.to_string())?
      .json::<Vec<PistonRuntime>>()
      .await
      .map_err(|e| e.to_string())?
      .iter()
      .map(|r| Runtime {
        language: r.language.to_owned(),
        version: r.version.to_owned(),
      })
      .collect_vec(),
  )
}

#[openapi]
#[get("/runtimes")]
async fn runtimes() -> Result<Json<Vec<Runtime>>, status::NotFound<String>> {
  piston_runtimes().await.map(Json).map_err(status::NotFound)
}

#[launch]
fn rocket() -> _ {
  rocket::custom(Config {
    port: PORT,
    ..Config::default()
  })
  .mount("/", openapi_get_routes![runtimes, execute])
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
