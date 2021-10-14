#[macro_use]
extern crate rocket;
use rayon::prelude::*;
use rocket::{
  response::status,
  serde::{json::Json, Deserialize, Serialize},
  Config,
};

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteRequest {
  language: String,
  version: String,
  code: String,
  test: Test,
}

#[derive(Serialize, Deserialize, Debug)]
struct Test {
  input: String,
  output: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunJson {
  stdout: String,
  stderr: String,
  code: Option<i64>,
  signal: Option<String>,
  output: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteJson {
  run: RunJson,
  language: String,
  version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RuntimeJson {
  language: String,
  version: String,
  aliases: Vec<String>,
  runtime: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileJson {
  name: Option<String>,
  content: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteResult {
  stdout: String,
  stderr: Option<String>,
  passed: bool,
}

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
    .expect("asd")
    .json::<ExecuteJson>()
    .await
    .expect("qwe");

  Ok(Json(ExecuteResult {
    stdout: res.run.stdout,
    stderr: if res.run.stderr == "" {
      None
    } else {
      Some(res.run.stderr)
    },
    passed: res.run.output.trim() == data.test.output.trim(),
  }))
}

#[get("/runtimes")]
async fn runtimes() -> Result<Json<Vec<RuntimeJson>>, status::NotFound<String>> {
  let runtimes_api = "http://localhost:2000/api/v2/runtimes";

  let res = reqwest::get(runtimes_api)
    .await
    .map_err(|e| status::NotFound(e.to_string()))?
    .json::<Vec<RuntimeJson>>()
    .await
    .map_err(|e| status::NotFound(e.to_string()))?;

  Ok(Json(res))
}

#[launch]
fn rocket() -> _ {
  rocket::custom(Config {
    port: 5000,
    ..Config::default()
  })
  .mount("/", routes![runtimes, submit])
}

// #[tokio::main]
// async fn main() {
//   let runtimes_api = "http://localhost:2000/api/v2/runtimes";
//   let execute_api = "http://localhost:2000/api/v2/execute";

//   let res = reqwest::get(runtimes_api)
//     .await
//     .expect("1")
//     .json::<Vec<RuntimeJson>>()
//     .await
//     .expect("2");

//   let post_json_str = r#"
//     {
//       "language": "ruby",
//       "version": "3.0.1",
//       "files": [
//         {
//           "content": "$><<`dd`.split.sum(&:to_i)"
//         }
//       ],
//       "stdin": "1\n2\n3",
//       "compile_timeout": 10000,
//       "run_timeout": 3000,
//       "compile_memory_limit": -1,
//       "run_memory_limit": -1
//     }"#;

//   let post_json: PostExecuteJson = serde_json::from_str(post_json_str).unwrap();

//   let res = reqwest::Client::new()
//     .post(execute_api)
//     .json(&post_json)
//     .send()
//     .await
//     .unwrap()
//     .json::<ExecuteJson>()
//     .await
//     .unwrap();

//   println!("{:#?}", res);
// }
