#[macro_use]
extern crate dotenv_codegen;

use cached::proc_macro::cached;
use itertools::Itertools;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::okapi::{schemars, schemars::JsonSchema};

const COMPILE_MEMORY_LIMIT: i64 = 512 * 1024 * 1024;
const RUN_MEMORY_LIMIT: i64 = 512 * 1024 * 1024;

const PISTON_API: &str = dotenv!("PISTON_API");

#[derive(Serialize, Deserialize)]
struct PistonJob {
  stdout: String,
  stderr: String,
  code: Option<i64>,
  signal: Option<String>,
  output: String,
  time: i64,
  time_limit_exceeded: bool,
}

#[derive(Serialize, Deserialize)]
struct PistonExecution {
  compile: Option<PistonJob>,
  run: PistonJob,
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

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct PistonMessageError {
  message: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCodeRequest {
  code: String,
  language: String,
  version: Option<String>,
  input: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
  stdout: String,
  stderr: Option<String>,
  time: i64,
  time_limit_exceeded: bool,
  did_not_crash: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
  language: String,
  version: String,
  aliases: Vec<String>,
}

fn empty_to_none(str: String) -> Option<String> {
  if str.is_empty() {
    None
  } else {
    Some(str)
  }
}

#[cached(time = 60, result = true)]
pub async fn piston_execute(data: ExecuteCodeRequest) -> Result<Execution, String> {
  let execute_json = PistonExecuteRequest {
    language: data.language.clone(),
    version: piston_runtimes()
      .await?
      .into_iter()
      .filter(|runtime| data.version.clone().map_or(true, |version| version == runtime.version))
      .find(|runtime| {
        runtime.language.to_lowercase() == data.language.to_lowercase()
          || runtime
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase() == data.language.to_lowercase())
      })
      .ok_or(format!(
        "Couldn't find '{}' language{}",
        data.language,
        data
          .version
          .map_or("".to_string(), |v| format!(" which has the '{}' version", v))
      ))?
      .version,
    args: data.input.clone().map(|s| s.lines().map(str::to_string).collect_vec()),
    compile_memory_limit: Some(COMPILE_MEMORY_LIMIT),
    compile_timeout: None,
    run_memory_limit: Some(RUN_MEMORY_LIMIT),
    run_timeout: None,
    stdin: data.input,
    files: vec![PistonFile {
      name: Some("Main".to_string()),
      content: data.code.clone(),
    }],
  };

  let response_value: serde_json::Value = reqwest::Client::new()
    .post(PISTON_API.to_string() + "/execute")
    .json(&execute_json)
    .send()
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())?;

  let res = serde_json::from_value::<PistonExecution>(response_value.clone()).map_err(|_| {
    serde_json::from_value::<PistonMessageError>(response_value)
      .map(|r| r.message)
      .unwrap_or_else(|e| e.to_string())
  })?;

  let stdout = if res.run.stdout.ends_with('\n') {
    // chop off \n if it ends with it
    res.run.stdout[0..res.run.stdout.len() - 1].to_string()
  } else {
    res.run.stdout.clone()
  };

  let compile_stderr = if let Some(compile) = &res.compile {
    empty_to_none(compile.stderr.clone())
  } else {
    None
  };

  let run_stderr = empty_to_none(res.run.stderr);

  let stderr = compile_stderr.or(run_stderr);

  let did_not_crash = res.run.code.map_or(res.run.signal.is_none(), |c| c == 0);

  Ok(Execution {
    stdout,
    stderr,
    time: res.run.time,
    time_limit_exceeded: res.run.time_limit_exceeded,
    did_not_crash,
  })
}

#[cached(time = 60, result = true)]
pub async fn piston_runtimes() -> Result<Vec<Runtime>, String> {
  Ok(
    reqwest::get(PISTON_API.to_string() + "/runtimes")
      .await
      .map_err(|e| e.to_string())?
      .json::<Vec<PistonRuntime>>()
      .await
      .map_err(|e| e.to_string())?
      .iter()
      .map(|r| Runtime {
        language: r.language.clone(),
        version: r.version.clone(),
        aliases: r.aliases.clone(),
      })
      .collect_vec(),
  )
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Test {
  input: String,
  expected_output: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunTests {
  code: String,
  language: String,
  version: Option<String>,
  tests: Vec<Test>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionWithTest {
  input: String,
  expected_output: String,
  actual_output: String,
  stderr: Option<String>,
  time: i64,
  time_limit_exceeded: bool,
  did_not_crash: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteWithTests {
  executions: Vec<ExecutionWithTest>,
  tests_passed: i64,
}

#[cached(time = 60, result = true)]
pub async fn execute_with_tests(data: RunTests) -> Result<ExecuteWithTests, String> {
  let mut executions: Vec<ExecutionWithTest> = Vec::new();

  let mut tests_passed = 0;

  for test in data.tests {
    let execution = piston_execute(ExecuteCodeRequest {
      code: data.code.clone(),
      input: Some(test.input.clone()),
      language: data.language.clone(),
      version: data.version.clone(),
    })
    .await?;

    if execution.stdout == test.expected_output && execution.did_not_crash {
      tests_passed += 1;
    }

    executions.push(ExecutionWithTest {
      input: test.input,
      actual_output: execution.stdout,
      expected_output: test.expected_output,
      did_not_crash: execution.did_not_crash,
      stderr: execution.stderr,
      time: execution.time,
      time_limit_exceeded: execution.time_limit_exceeded,
    });
  }

  Ok(ExecuteWithTests {
    executions,
    tests_passed,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test()]
  async fn execute_ruby() -> Result<(), String> {
    let execution = piston_execute(ExecuteCodeRequest {
      code: "p $*.sum &:to_i".to_string(),
      language: "ruby".to_string(),
      version: None,
      input: Some("1\n2".to_string()),
    })
    .await?;

    if execution.stdout != "3" {
      return Err("Output is not equal to six".to_string());
    }

    Ok(())
  }
}
