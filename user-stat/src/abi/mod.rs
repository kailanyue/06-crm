use chrono::{DateTime, TimeZone, Utc};
use itertools::Itertools;
use prost_types::Timestamp;
use tonic::{Response, Status};
use tracing::info;

use crate::{
    pb::{QueryRequest, RawQueryRequest, User},
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
        let Ok(ret) = sqlx::query_as::<_, User>(&req.query)
            .fetch_all(&self.inner.pool)
            .await
        else {
            return Err(Status::internal(format!(
                "Failed to fetch data with query: {}",
                req.query
            )));
        };

        Ok(Response::new(Box::pin(futures::stream::iter(
            ret.into_iter().map(Ok),
        ))))
    }
}

fn generate_sql(time_conditions: &str, id_conditions: &str) -> String {
    let mut sql = "SELECT email, name FROM user_stats WHERE ".to_string();

    if !time_conditions.is_empty() {
        sql.push_str(time_conditions);
        sql.push_str(" AND ");
    }

    if !id_conditions.is_empty() {
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

fn ts_to_utc(ts: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}

fn ids_query(name: &str, ids: &Vec<u32>) -> String {
    if ids.is_empty() {
        return "TRUE".to_string();
    }

    format!("array{:?} <@ {}", ids, name)
}

#[cfg(test)]
mod tests {
    use crate::{
        pb::{IdQuery, QueryRequestBuilder, TimeQuery},
        AppConfig,
    };
    use anyhow::Result;
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    async fn raw_query_should_work() -> Result<()> {
        let config = AppConfig::load().expect("Failed to load config");

        let service = UserStatsService::new(config).await;

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
        let config = AppConfig::load().expect("Failed to load config");
        let service = UserStatsService::new(config).await;

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

    fn id(id: &[u32]) -> IdQuery {
        IdQuery { ids: id.to_vec() }
    }

    fn tq(lower: Option<i64>, upper: Option<i64>) -> TimeQuery {
        TimeQuery {
            lower: lower.map(days_to_ts),
            upper: upper.map(days_to_ts),
        }
    }

    fn days_to_ts(days: i64) -> Timestamp {
        let dt = Utc::now()
            .checked_sub_signed(chrono::Duration::days(days))
            .unwrap();
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}
