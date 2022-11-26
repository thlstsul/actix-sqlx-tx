use std::{marker::PhantomData, rc::Rc};

use actix_utils::future::{ready, Ready};
use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    HttpMessage, ResponseError,
};
use futures_core::future::LocalBoxFuture;

use crate::{tx::TxSlot, Error};

/// This middleware adds a lazily-initialised transaction to the [request extensions]. The first time the
/// [`Tx`] extractor is used on a request, a connection is acquired from the configured
/// [`sqlx::Pool`] and a transaction is started on it. The same transaction will be returned for
/// subsequent uses of [`Tx`] on the same request. The inner service is then called as normal. Once
/// the inner service responds, the transaction is committed or rolled back depending on the status
/// code of the response.
///
/// [`Tx`]: crate::Tx
/// [request extensions]: https://docs.rs/actix-web/latest/actix_web/dev/struct.Extensions.html
/// [refer to axum-sqlx-tx]: https://github.com/wasdacraic/axum-sqlx-tx
#[derive(Clone)]
pub struct TransactionMiddleware<DB: sqlx::Database, E = Error> {
    pool: Rc<sqlx::Pool<DB>>,
    _error: PhantomData<E>,
}

impl<DB: sqlx::Database> TransactionMiddleware<DB> {
    pub fn new(pool: sqlx::Pool<DB>) -> Self {
        Self::new_with_error(pool)
    }

    /// Construct a new layer with a specific error type.
    pub fn new_with_error<E>(pool: sqlx::Pool<DB>) -> TransactionMiddleware<DB, E> {
        TransactionMiddleware {
            pool: Rc::new(pool),
            _error: PhantomData,
        }
    }
}

impl<S, B, DB, E> Transform<S, ServiceRequest> for TransactionMiddleware<DB, E>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    DB: sqlx::Database + 'static,
    E: From<Error> + ResponseError + 'static,
{
    type Response = ServiceResponse<B>;

    type Error = actix_web::Error;

    type Transform = InnerTransactionMiddleware<S, DB, E>;

    type InitError = ();

    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(InnerTransactionMiddleware {
            service: Rc::new(service),
            pool: Rc::clone(&self.pool),
            _error: self._error,
        }))
    }
}

#[doc(hidden)]
pub struct InnerTransactionMiddleware<S, DB: sqlx::Database, E = Error> {
    service: Rc<S>,
    pool: Rc<sqlx::Pool<DB>>,
    _error: PhantomData<E>,
}

impl<DB: sqlx::Database, S: Clone, E> Clone for InnerTransactionMiddleware<S, DB, E> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            service: self.service.clone(),
            _error: self._error,
        }
    }
}

impl<S, B, DB, E> Service<ServiceRequest> for InnerTransactionMiddleware<S, DB, E>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    DB: sqlx::Database + 'static,
    E: From<Error> + ResponseError + 'static,
{
    type Response = ServiceResponse<B>;

    type Error = actix_web::Error;

    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let transaction = TxSlot::bind(&mut req.extensions_mut(), &self.pool);
        let srv = Rc::clone(&self.service);
        let res = srv.call(req);

        Box::pin(async move {
            let res = res.await?;

            if res.status().is_success() {
                if let Err(error) = transaction.commit().await {
                    return Err(E::from(Error::Database { error }).into());
                }
            }
            Ok(res)
        })
    }
}
