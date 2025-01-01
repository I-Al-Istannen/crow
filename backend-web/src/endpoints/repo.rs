use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::WebError;
use crate::types::{AppState, Repo, TeamId};
use axum::extract::{Path, State};

pub async fn get_repo(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(team_id): Path<TeamId>,
) -> Result<Json<Repo>, WebError> {
    let user = db.get_user(&claims.sub).await?;

    if claims.is_admin() {
        return Ok(Json(db.get_repo(&team_id).await?));
    }

    let Some(team) = user.user.team else {
        return Err(WebError::NotInTeam);
    };

    Ok(Json(db.get_repo(&team).await?))
}
