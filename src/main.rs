// use axum::Router;

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, RwLock, RwLockReadGuard},
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
        .route("/reveal/:id", get(reveal_handler_2))
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

async fn reveal_handler_2(
    Path(id): Path<String>,
    Extension(state): Extension<SharedAppDBState>,
) -> Result<String, RefinedMainError> {
    let y = read_from_hash(&state).change_context(MainError::UnknownCause)?;
    // .and_then(|hash_value| get_guess(&hash_value, id))
    // .map(|guess| guess.clone().to_string());

    let y = get_guess(&y, id)?;

    Ok(y.to_string())
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
        .map_err(|_s| CustomErrors::InvalidGame {
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

fn read_from_hash(
    shared_hash: &SharedAppDBState,
) -> Result<RwLockReadGuard<AppDBState>, Report<LowLevelErrors>> {
    if let Ok(read_guard) = shared_hash.read() {
        // the returned read_guard also implements `Deref`
        Ok(read_guard)
    } else {
        Err(LowLevelErrors::PoisonedHash)
            .report()
            .change_context(LowLevelErrors::PoisonedHash)
        // shared_hash.clone().read().map_err(|_err| MainError::UnknownCause).report().change_context(LowLevelErrors::PoisonedHash)
    }
}

fn get_guess<'a>(
    shared_hash: &'a RwLockReadGuard<'a, AppDBState>,
    id: String,
) -> Result<&'a u32, Report<MainError>> {
    shared_hash
        .guess_pairs
        .get(&id.clone())
        // .map(|x| x.clone())
        .ok_or_else(|| LowLevelErrors::PoisonedHash)
        .report()
        .change_context(MainError::InvalidGame {
            game_id: id.to_string(),
        })
}

#[derive(ErrorStack, Debug, Default)]
enum MainError {
    #[error_message(&format!("The game :: {:?} is not valid", game_id))]
    InvalidGame { game_id: String },
    #[error_message(&format!("No guess available for {:?} game", game_id))]
    NoGuessvalueForId { game_id: String },
    #[default]
    #[error_message(&format!("Error not known but we crashed"))]
    UnknownCause,
}
#[derive(ErrorStack, Debug)]
enum LowLevelErrors {
    PoisonedHash,
}

struct RefinedMainError(Report<MainError>);

impl IntoResponse for RefinedMainError {
    fn into_response(self) -> Response {
        // let y = self.0.downcast_ref::<CustomErrors>().unwrap();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error :: {}", self.0.to_string()),
        )
            .into_response()
    }
}

impl<E> From<E> for RefinedMainError
where
    E: Into<Report<MainError>>,
{
    fn from(_err: E) -> Self {
        RefinedMainError(_err.into())
    }
}
