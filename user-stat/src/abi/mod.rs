use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use itertools::Itertools;
use prost_types::Timestamp;
use sqlx::Row;
use tonic::{Response, Status};
use tracing::info;

use crate::{
    pb::{QueryRequest, QueryRequestBuilder, RawQueryRequest, TimeQuery, User},
    ResponseStream, ServiceResult, UserStatsService,
};

impl UserStatsService {
    pub async fn query(&self, query: QueryRequest) -> ServiceResult<ResponseStream> {
        let time_conditions = generate_time_conditions(&query);
        let id_conditions = generate_id_conditions(&query);
        let sql = generate_sql(&time_conditions, &id_conditions);

        info!("Generated SQL: {}", sql);
        self.raw_query(RawQueryRequest { query: sql }).await
    }

    pub async fn raw_query(&self, req: RawQueryRequest) -> ServiceResult<ResponseStream> {
        let Ok(rows) = sqlx::query(&req.query).fetch_all(&self.inner.pool).await else {
            return Err(Status::internal(format!(
                "Failed to fetch data with query: {}",
                req.query
            )));
        };

        let ret: Vec<User> = rows
            .into_iter()
            .map(|row| {
                let email: String = row
                    .try_get("email")
                    .map_err(|e| Status::internal(format!("Failed to get email: {}", e)))?;

                let name: String = row
                    .try_get("name")
                    .map_err(|e| Status::internal(format!("Failed to get name: {}", e)))?;

                let viewed_but_not_started: Vec<i32> =
                    row.try_get("viewed_but_not_started").map_err(|e| {
                        Status::internal(format!("Failed to get viewed_but_not_started: {}", e))
                    })?;

                let started_but_not_finished: Vec<i32> =
                    row.try_get("started_but_not_finished").map_err(|e| {
                        Status::internal(format!("Failed to get started_but_not_finished: {}", e))
                    })?;

                Ok(User {
                    email,
                    name,
                    viewed_but_not_started: viewed_but_not_started
                        .into_iter()
                        .map(|i| i as i64)
                        .collect(),
                    started_but_not_finished: started_but_not_finished
                        .into_iter()
                        .map(|i| i as i64)
                        .collect(),
                })
            })
            .collect::<Result<Vec<User>, Status>>()?;

        Ok(Response::new(Box::pin(futures::stream::iter(
            ret.into_iter().map(Ok),
        ))))
    }
}

fn generate_sql(time_conditions: &str, id_conditions: &str) -> String {
    let mut sql = r#"SELECT email, name, viewed_but_not_started, started_but_not_finished FROM user_stats WHERE "#
        .to_string();

    if !time_conditions.is_empty() {
        sql.push_str(time_conditions);
    }

    if !id_conditions.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(id_conditions);
    }

    info!("Generated SQL: {}", sql);
    sql
}

fn generate_time_conditions(query: &QueryRequest) -> String {
    query
        .timestamps
        .iter()
        .map(|(k, v)| timestamp_query(k, &v.lower, &v.upper))
        .join(" AND ")
}

fn generate_id_conditions(query: &QueryRequest) -> String {
    query
        .ids
        .iter()
        .map(|(k, v)| ids_query(k, &v.ids))
        .join(" AND ")
}

fn timestamp_query(name: &str, lower: &Option<Timestamp>, upper: &Option<Timestamp>) -> String {
    match (lower, upper) {
        (None, None) => "TRUE".to_string(),
        (None, Some(upper)) => format!("{} <= '{}'", name, ts_to_utc(upper).to_rfc3339()),
        (Some(lower), None) => format!("{} >= '{}'", name, ts_to_utc(lower).to_rfc3339()),
        (Some(lower), Some(upper)) => format!(
            "{} BETWEEN '{}' AND '{}'",
            name,
            ts_to_utc(lower).to_rfc3339(),
            ts_to_utc(upper).to_rfc3339()
        ),
    }
}

impl QueryRequest {
    pub fn new_with_dt(name: &str, lower: DateTime<Utc>, upper: DateTime<Utc>) -> Self {
        let ts = Timestamp {
            seconds: lower.timestamp(),
            nanos: 0,
        };
        let ts1 = Timestamp {
            seconds: upper.timestamp(),
            nanos: 0,
        };
        let tq = TimeQuery {
            lower: Some(ts),
            upper: Some(ts1),
        };

        QueryRequestBuilder::default()
            .timestamp((name.to_string(), tq))
            .build()
            .expect("Failed to build query request")
    }
}

fn ts_to_utc(ts: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}

fn ids_query(name: &str, ids: &Vec<u32>) -> String {
    if ids.is_empty() {
        return "TRUE".to_string();
    }

    format!("array{:?} <@ {}", ids, name)
}

impl fmt::Display for QueryRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_conditions = generate_time_conditions(self);
        let id_conditions = generate_id_conditions(self);
        let sql = generate_sql(&time_conditions, &id_conditions);

        info!("Generated SQL: {}", sql);

        write!(f, "{}", sql)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        pb::QueryRequestBuilder,
        test_utils::{id, tq},
    };
    use anyhow::Result;
    use futures::StreamExt;

    use super::*;

    #[test]
    fn query_request_to_string_should_work() {
        let d1 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        let query = QueryRequest::new_with_dt("created_at", d1, d2);
        let sql = query.to_string();
        assert_eq!(
            sql,
            "SELECT email, name, viewed_but_not_started, started_but_not_finished FROM user_stats WHERE created_at BETWEEN '2024-01-01T00:00:00+00:00' AND '2024-01-02T00:00:00+00:00'"
        );
    }

    #[tokio::test]
    async fn raw_query_should_work() -> Result<()> {
        let (_tbd, service) = UserStatsService::new_for_test().await?;

        let mut stream = service
            .raw_query(RawQueryRequest {
                query: "select * from user_stats where created_at > '2024-01-01' limit 5"
                    .to_string(),
            })
            .await?
            .into_inner();

        while let Some(res) = stream.next().await {
            println!("{:?}", res);
        }
        Ok(())
    }

    #[tokio::test]
    async fn query_should_work() -> Result<()> {
        let (_tbd, service) = UserStatsService::new_for_test().await?;

        let query = QueryRequestBuilder::default()
            .timestamp(("created_at".to_string(), tq(Some(120), None)))
            .timestamp(("last_visited_at".to_string(), tq(Some(30), None)))
            .id(("viewed_but_not_started".to_string(), id(&[252790])))
            .build()
            .unwrap();

        let mut stream = service.query(query).await?.into_inner();

        while let Some(res) = stream.next().await {
            println!("{:?}", res);
        }

        Ok(())
    }
}
