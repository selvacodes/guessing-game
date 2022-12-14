// use axum::Router;

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router, Server as A_Server,
};
use error_stack_derive::ErrorStack;
use rand::prelude::*;
use tokio::{self};
use uuid::Uuid;

// use error_stack::{Context, IntoReport, Report, Result, ResultExt};

use error_stack::{IntoReport, Report, ResultExt};

#[derive(ErrorStack, Debug, Default)]
#[error_message(&format!("Error occured with Guess2 "))]
struct GuessError2;

#[derive(ErrorStack, Debug, Default)]
#[error_message(&format!("Error occured with foo "))]
struct GuessError;

fn infer_lifetime<'a, T: 'a, F: Fn(&'a T) -> &'a T>(f: F) -> F {
    f
}

// the application state
#[derive(Clone)]
struct AppState {
    // that holds some api specific state
    number_to_guess: u32,
}

#[derive(Clone, Default)]
struct AppDBState {
    guess_pairs: HashMap<String, u32>,
}

type SharedAppDBState = Arc<RwLock<AppDBState>>;

struct AppError(GuessError2);

impl IntoResponse for GuessError2 {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self),
        )
            .into_response()
    }
}

impl From<GuessError2> for AppError {
    fn from(g: GuessError2) -> Self {
        AppError(g)
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let shared_state: AppState = AppState {
        number_to_guess: 10,
    };

    let shared_db_state: SharedAppDBState = SharedAppDBState::default();

    // build our application with a route
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                // println!("Service alive");
                format!("Service alive")
            }),
        )
        .route("/generate-game-id", get(generate_game_handler))
        .route("/reveal/:id", get(reveal_handler))
        .route("/guess/:id/:guess", get(guess_num_handler))
        .layer(Extension(shared_db_state))
        .into_make_service();

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    // tracing::debug!("listening on {}", addr);
    A_Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app)
        .await
        .unwrap();
}

async fn generate_game_handler(Extension(state): Extension<SharedAppDBState>) -> String {
    let id = Uuid::new_v4();
    let mut rng = rand::thread_rng();
    let gen_num: u32 = rng.gen_range(0..20);
    state
        .write()
        .unwrap()
        .guess_pairs
        .insert(id.to_string(), gen_num);
    id.to_string()
}

async fn reveal_handler(
    Path(id): Path<String>,
    Extension(state): Extension<SharedAppDBState>,
) -> Result<String, GuessError2> {
    let y = state
        .read()
        .map_err(|s| GuessError2)
        .report()
        .change_context(GuessError)?;

    let y = y.guess_pairs.get(&id).ok_or_else(|| GuessError).report()?;

    Ok(y.to_string())
}

async fn guess_num_handler(
    Path((id, guess)): Path<(String, u32)>,
    // Path(guess): Path<u32>,
    Extension(state): Extension<SharedAppDBState>,
) -> Result<String, RefinedError> {
    let y = state
        .read()
        .map_err(|s| CustomErrors::InvalidGame {
            game_id: id.clone(),
        })
        .report()?;

    let y1 = y
        .guess_pairs
        .get(&id)
        .ok_or_else(|| CustomErrors::GuessDataMissingForGame {
            game_id: id.clone(),
        })
        .report()?;

    let y2 = y1.to_owned();
    if y2 == guess {
        Ok(format!("YOur guess is correct {y2}"))
    } else if y2 < guess {
        Ok(format!("YOur guess is greater"))
    } else {
        Ok(format!("YOur guess is lesser"))
    }
}

async fn hello_world_handler() -> &'static str {
    "Hello Selva"
}

async fn handler(Extension(state): Extension<AppState>) -> String {
    let y = state.number_to_guess;
    format!("THE NUMBER IS :: {y}")
}

async fn guess_handler(Path(user_id): Path<u32>, Extension(state): Extension<AppState>) -> String {
    let y: u32 = state.number_to_guess;

    if y == user_id {
        format!("YOur guess is correct {y}")
    } else {
        format!("YOur guess is wrong")
    }
}

// fn parse_config(path: impl AsRef<Path>) -> Result<Config, Report<ParseConfigError>> {
//     let path = path.as_ref();

//     // First, we have a base error:
//     let io_result = fs::read_to_string(path);      // Result<File, std::io::Error>

//     // Convert the base error into a Report:
//     let io_report = io_result.report();            // Result<File, Report<std::io::Error>>

//     // Change the context, which will eventually return from the function:
//     let config_report = io_report
//         .change_context(ParseConfigError::new())?; // Result<File, Report<ParseConfigError>>

//     // ...
// }

impl<E> From<E> for GuessError2
where
    E: Into<Report<GuessError>>,
{
    fn from(_err: E) -> Self {
        Self::default()
    }
}

#[derive(Debug, ErrorStack, Default, Clone)]
#[error_message(&format!("Something went haywire"))]
enum CustomErrors {
    InvalidGame {
        game_id: String,
    },
    GuessDataMissingForGame {
        game_id: String,
    },
    #[default]
    OtherError,
}

struct RefinedError(Report<CustomErrors>);

impl IntoResponse for RefinedError {
    fn into_response(self) -> Response {
        // let y = self.0.downcast_ref::<CustomErrors>().unwrap();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error :: {}", self.0.to_string()),
        )
            .into_response()

        // match y.clone() {
        //     (CustomErrors::OtherError) => (
        //         StatusCode::INTERNAL_SERVER_ERROR,
        //         format!("Something went wrong"),
        //     ),
        //     (CustomErrors::GuessDataMissingForGame { game_id }) => (
        //         StatusCode::INTERNAL_SERVER_ERROR,
        //         format!("Data missing for {}", game_id.clone()),
        //     ),
        //     (CustomErrors::InvalidGame { game_id }) => (
        //         StatusCode::INTERNAL_SERVER_ERROR,
        //         format!("Wrong Game Id :: {}", game_id.clone()),
        //     ),
        // }
        // .into_response()

        // match self.0 {

        // }
        // .into_response()
    }
}

impl<E> From<E> for RefinedError
where
    E: Into<Report<CustomErrors>>,
{
    fn from(_err: E) -> Self {
        RefinedError(_err.into())
    }
}
