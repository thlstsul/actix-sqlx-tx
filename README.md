# `actix-sqlx-tx`

Request-bound [SQLx](https://github.com/launchbadge/sqlx) transactions for [actix-web](https://github.com/actix/actix-web);
Refer to [axum-sqlx-tx](https://github.com/wasdacraic/axum-sqlx-tx).

## Summary

`actix-sqlx-tx` provides an `actix-web` [middleware](https://actix.rs/docs/middleware) for obtaining a request-bound transaction.
The transaction begins the first time the middleware is used, and is stored with the request for use by other middleware/handlers.
The transaction is resolved depending on the status code of the response – successful (`2XX`) responses will commit the transaction, otherwise it will be rolled back.

See the [crate documentation](https://docs.rs/actix-sqlx-tx) for more information and examples.