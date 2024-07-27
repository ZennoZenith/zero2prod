use sqlx::{Pool, Postgres};

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[sqlx::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard(pool: Pool<Postgres>) {
    // Arrange
    let app = spawn_app(pool).await;
    // Act
    let response = app.get_admin_dashboard().await;
    // Assert
    assert_is_redirect_to(&response, "/login");
}
