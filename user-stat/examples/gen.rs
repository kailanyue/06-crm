use core::fmt;
use std::fmt::Write;
use std::time::Duration;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
}; // For `write!` macro

use anyhow::Result;
use chrono::{DateTime, Days, Utc};
use fake::{
    faker::{chrono::en::DateTimeBetween, internet::en::SafeEmail, name::zh_cn::Name},
    Dummy, Fake, Faker,
};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgPool};
use tokio::task;
use tokio::time::{sleep, Instant};
use user_stat::AppConfig;

// generate 10000 users and run them in a tx, repeat 500 times
#[derive(Debug, Clone, Dummy, Serialize, Deserialize, PartialEq, Eq)]
enum Gender {
    Male,
    Female,
    Unknown,
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let gender_str = match self {
            Gender::Male => "male",
            Gender::Female => "female",
            Gender::Unknown => "unknown",
        };
        write!(f, "{}", gender_str)
    }
}

#[derive(Debug, Clone, Dummy, Serialize, Deserialize, PartialEq, Eq)]
struct UserStat {
    #[dummy(faker = "UniqueEmail")]
    email: String,
    #[dummy(faker = "Name()")]
    name: String,
    #[dummy(faker = "GenderGenerator")]
    gender: Gender,

    #[dummy(faker = "DateTimeBetween(before(365*5), before(90))")]
    created_at: DateTime<Utc>,
    #[dummy(faker = "DateTimeBetween(before(30), now())")]
    last_visited_at: DateTime<Utc>,
    #[dummy(faker = "DateTimeBetween(before(90), now())")]
    last_watched_at: DateTime<Utc>,

    #[dummy(faker = "IntList(50, 100000, 100000)")]
    recent_watched: Vec<i32>,
    #[dummy(faker = "IntList(50, 200000, 100000)")]
    viewed_but_not_started: Vec<i32>,
    #[dummy(faker = "IntList(50, 300000, 100000)")]
    started_but_not_finished: Vec<i32>,
    #[dummy(faker = "IntList(50, 400000, 100000)")]
    finished: Vec<i32>,

    #[dummy(faker = "DateTimeBetween(before(45), now())")]
    last_email_notification: DateTime<Utc>,
    #[dummy(faker = "DateTimeBetween(before(15), now())")]
    last_in_app_notification: DateTime<Utc>,
    #[dummy(faker = "DateTimeBetween(before(90), now())")]
    last_sms_notification: DateTime<Utc>,
}

impl Hash for UserStat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.email.hash(state);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    let pool = PgPool::connect(&config.server.db_url).await?;
    for i in 1..=500 {
        let users: HashSet<_> = (0..10000).map(|_| Faker.fake::<UserStat>()).collect();

        let start = Instant::now();
        raw_insert(users, &pool).await?;
        println!("Batch {} inserted in {:?}", i, start.elapsed());
        if i % 10 == 0 {
            println!("Batch {} sleeping", i);
            sleep(Duration::from_secs(5)).await;
        }
    }
    Ok(())
}

fn list_to_string(list: Vec<i32>) -> String {
    format!("ARRAY{:?}", list)
}

#[allow(dead_code)]
async fn raw_insert1(users: HashSet<UserStat>, pool: &PgPool) -> Result<(), sqlx::Error> {
    let batch_size = 1000; // 每次批量插入的大小
    let users: Vec<UserStat> = users.into_iter().collect();
    let total_batches = (users.len() + batch_size - 1) / batch_size; // 计算总批次数

    let mut tasks = Vec::new();

    for batch_index in 0..total_batches {
        let pool = pool.clone();
        let start = batch_index * batch_size;
        let end = ((batch_index + 1) * batch_size).min(users.len());
        let batch: Vec<UserStat> = users[start..end].to_vec();

        let task = task::spawn(async move {
            let mut sql = String::with_capacity(10 * 1000 * 1000);
            sql.push_str("
            INSERT INTO user_stats(email, name, gender, created_at, last_visited_at, last_watched_at, recent_watched, viewed_but_not_started, started_but_not_finished, finished, last_email_notification, last_in_app_notification, last_sms_notification)
            VALUES");

            for user in batch {
                write!(
                    sql,
                    "('{}', '{}', '{}', '{}', '{}', '{}', {}::int[], {}::int[], {}::int[], {}::int[], '{}', '{}', '{}'),",
                    user.email,
                    user.name,
                    user.gender, // Gender enum to string
                    user.created_at,
                    user.last_visited_at,
                    user.last_watched_at,
                    list_to_string(user.recent_watched),
                    list_to_string(user.viewed_but_not_started),
                    list_to_string(user.started_but_not_finished),
                    list_to_string(user.finished),
                    user.last_email_notification,
                    user.last_in_app_notification,
                    user.last_sms_notification,
                ).unwrap();
            }

            sql.pop(); // Remove the trailing comma

            sqlx::query(&sql).execute(&pool).await
        });

        tasks.push(task);
    }

    // 等待所有任务完成
    for task in tasks {
        task.await.unwrap()?;
    }

    Ok(())
}

async fn raw_insert(users: HashSet<UserStat>, pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut sql = String::with_capacity(10 * 1000 * 1000);
    sql.push_str("
    INSERT INTO user_stats(email, name, gender, created_at, last_visited_at, last_watched_at, recent_watched, viewed_but_not_started, started_but_not_finished, finished, last_email_notification, last_in_app_notification, last_sms_notification)
    VALUES");

    for user in users {
        write!(
            sql,
            "('{}', '{}', '{}', '{}', '{}', '{}', {}::int[], {}::int[], {}::int[], {}::int[], '{}', '{}', '{}'),",
            user.email,
            user.name,
            user.gender, // Gender enum to string
            user.created_at,
            user.last_visited_at,
            user.last_watched_at,
            list_to_string(user.recent_watched),
            list_to_string(user.viewed_but_not_started),
            list_to_string(user.started_but_not_finished),
            list_to_string(user.finished),
            user.last_email_notification,
            user.last_in_app_notification,
            user.last_sms_notification,
        ).unwrap();
    }

    sql.pop(); // Remove the trailing comma

    sqlx::query(&sql).execute(pool).await?;

    Ok(())
}

#[allow(dead_code)]
async fn bulk_insert(users: HashSet<UserStat>, pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    for user in users {
        let query = sqlx::query(
    r#"
        INSERT INTO user_stats(email, name, created_at, last_visited_at, last_watched_at, recent_watched, viewed_but_not_started, started_but_not_finished, finished, last_email_notification, last_in_app_notification, last_sms_notification)
        VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
        )
        .bind(&user.email)
        .bind(&user.name)
        .bind(user.created_at)
        .bind(user.last_visited_at)
        .bind(user.last_watched_at)
        .bind(&user.recent_watched)
        .bind(&user.viewed_but_not_started)
        .bind(&user.started_but_not_finished)
        .bind(&user.finished)
        .bind(user.last_email_notification)
        .bind(user.last_in_app_notification)
        .bind(user.last_sms_notification)
        ;
        tx.execute(query).await?;
    }
    tx.commit().await?;
    Ok(())
}

fn before(days: u64) -> DateTime<Utc> {
    Utc::now().checked_sub_days(Days::new(days)).unwrap()
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

struct IntList(pub i32, pub i32, pub i32);

impl Dummy<IntList> for Vec<i32> {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(v: &IntList, rng: &mut R) -> Self {
        let (max, start, len) = (v.0, v.1, v.2);
        let size = rng.gen_range(0..max);

        (0..size)
            .map(|_| rng.gen_range(start..start + len))
            .collect()
    }
}

const ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

struct UniqueEmail;

impl Dummy<UniqueEmail> for String {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &UniqueEmail, rng: &mut R) -> Self {
        let email: String = SafeEmail().fake_with_rng(rng);
        let id = nanoid!(8, &ALPHABET);
        let at = email.find('@').unwrap();
        format!("{}.{}{}", &email[..at], id, &email[at..])
    }
}

struct GenderGenerator;
impl Dummy<GenderGenerator> for Gender {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &GenderGenerator, rng: &mut R) -> Self {
        match rng.gen_range(0..3) {
            0 => Gender::Male,
            1 => Gender::Female,
            _ => Gender::Unknown,
        }
    }
}
