#[macro_use]
extern crate rocket;
use rocket::{
  response::status::NotFound,
  serde::{json::Json, Deserialize, Serialize},
  Config,
};

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

#[get("/runtimes")]
async fn runtimes() -> Result<Json<Vec<RuntimeJson>>, NotFound<String>> {
  let runtimes_api = "http://localhost:2000/api/v2/runtimes";

  let res = reqwest::get(runtimes_api)
    .await
    .map_err(|_| NotFound("1".to_string()))?
    .json::<Vec<RuntimeJson>>()
    .await
    .map_err(|_| NotFound("2".to_string()))?;

  Ok(Json(res))
}

#[launch]
fn rocket() -> _ {
  rocket::custom(Config {
    port: 5000,
    ..Config::default()
  })
  .mount("/", routes![runtimes])
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
