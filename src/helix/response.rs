//! Responses contains the return values of a [request](super::Request).
use super::{Cursor, Request};

/// Response retrieved from endpoint. Data is the type in [`Request::Response`]
#[derive(PartialEq, Eq, Debug)]
#[non_exhaustive]
pub struct Response<R, D>
where
    R: Request,
    D: serde::de::DeserializeOwned + PartialEq, {
    /// Twitch's response field for `data`.
    pub data: D,
    /// A cursor value, to be used in a subsequent request to specify the starting point of the next set of results.
    pub pagination: Option<Cursor>,
    /// The request that was sent, used for [pagination](super::Paginated).
    pub request: Option<R>,
    /// Response would return this many results if fully paginated. Sometimes this is not emmitted or correct for this purpose, in those cases, this value will be `None`.
    pub total: Option<i64>,
    /// Fields which are not part of the data response, but are returned by the endpoint.
    ///
    /// See for example [Get Broadcaster Subscriptions](https://dev.twitch.tv/docs/api/reference#get-broadcaster-subscriptions) which returns this.
    pub other: Option<serde_json::Map<String, serde_json::Value>>,
}

impl<R, D> Response<R, D>
where
    R: Request,
    D: serde::de::DeserializeOwned + PartialEq,
{
    /// Get a field from the response that is not part of `data`.
    pub fn get_other<Q, V>(&self, key: &Q) -> Result<Option<V>, serde_json::Error>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + std::hash::Hash,
        V: serde::de::DeserializeOwned, {
        use std::borrow::Borrow as _;
        match &key {
            total if &String::from("total").borrow() == total => {
                if let Some(total) = self.total {
                    let total = serde_json::json!(total);
                    Some(serde_json::from_value(total)).transpose()
                } else {
                    Ok(None)
                }
            }
            _ => self
                .other
                .as_ref()
                .and_then(|map| map.get(key.borrow()))
                .map(|v| serde_json::from_value(v.clone()))
                .transpose(),
        }
    }
}

impl<R, D, T> Response<R, D>
where
    R: Request,
    D: IntoIterator<Item = T> + PartialEq + serde::de::DeserializeOwned,
{
    /// Get first result of this response.
    pub fn first(self) -> Option<T> { self.data.into_iter().next() }
}

// impl<R, D, T> CustomResponse<'_, R, D>
// where
//     R: Request,
//     D: IntoIterator<Item = T>,
// {
//     /// Get first result of this response.
//     pub fn first(self) -> Option<T> { self.data().into_iter().next() }
// }

#[cfg(feature = "client")]
impl<R, D> Response<R, D>
where
    R: Request<Response = D> + Clone + super::Paginated + super::RequestGet + std::fmt::Debug,
    D: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    /// Get the next page in the responses.
    pub async fn get_next<'a, C: crate::HttpClient + 'a>(
        self,
        client: &'a super::HelixClient<'a, C>,
        token: &(impl super::TwitchToken + ?Sized),
    ) -> Result<Option<Response<R, D>>, super::ClientRequestError<<C as crate::HttpClient>::Error>>
    {
        if let Some(mut req) = self.request.clone() {
            if self.pagination.is_some() {
                req.set_pagination(self.pagination);
                let res = client.req_get(req, token).await.map(Some);
                if let Ok(Some(r)) = res {
                    // FIXME: Workaround for https://github.com/twitchdev/issues/issues/18
                    if r.data == self.data {
                        Ok(None)
                    } else {
                        Ok(Some(r))
                    }
                } else {
                    res
                }
            } else {
                Ok(None)
            }
        } else {
            // TODO: Make into proper error
            Err(super::ClientRequestError::Custom(
                "no source request attached".into(),
            ))
        }
    }
}
