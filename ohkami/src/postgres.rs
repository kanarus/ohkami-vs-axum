use futures_util::{stream::FuturesUnordered, TryStreamExt};
use rand::{rngs::SmallRng, SeedableRng, Rng, thread_rng};
use sqlx::{Statement, postgres::PgStatement};
use crate::models::{World, Fortune};

#[derive(Clone)]
pub struct Postgres {
    pool:       sqlx::PgPool,
    statements: TechEmpowerPostgresStatements,
}

#[derive(Clone)]
struct TechEmpowerPostgresStatements {
    select_world_by_id:            PgStatement<'static>,
    select_world_by_id_for_update: PgStatement<'static>,
    select_all_fortunes:           PgStatement<'static>,
    update_worlds:                 PgStatement<'static>,
}

impl Postgres {
    pub async fn new() -> Self {
        use sqlx::Executor as _;

        macro_rules! load_env {
            ($($name:ident as $t:ty)*) => {$(
                #[allow(non_snake_case)]
                let $name = ::std::env::var(stringify!($name))
                    .expect(concat!(
                        "failed to load environment variable ",
                        "`", stringify!($name), "`"
                    ))
                    .parse::<$t>()
                    .unwrap();
            )*};
        } load_env! {
            MAX_CONNECTIONS as u32
            MIN_CONNECTIONS as u32
            DATABASE_URL    as String
        }
            
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .min_connections(MIN_CONNECTIONS)
            .connect(&DATABASE_URL).await
            .unwrap();
        
        let statements = TechEmpowerPostgresStatements {
            select_world_by_id: pool
                .prepare("SELECT id, randomnumber FROM world WHERE id = $1 LIMIT 1")
                .await
                .unwrap(),
            select_world_by_id_for_update: pool
                .prepare("SELECT id, randomnumber FROM world WHERE id = $1 LIMIT 1 FOR UPDATE")
                .await
                .unwrap(),
            select_all_fortunes: pool
                .prepare("SELECT id, message FROM fortune")
                .await
                .unwrap(),
            update_worlds: pool
                .prepare("\
                    UPDATE world SET randomnumber = new.randomnumber FROM ( \
                        SELECT * FROM UNNEST($1::int[], $2::int[]) AS v(id, randomnumber) \
                        ORDER BY randomnumber \
                    ) AS new \
                    WHERE world.id = new.id \
                ")
                .await
                .unwrap(),
        };

        Self { pool, statements }
    }
}

impl Postgres {
    const ID_RANGE: std::ops::Range<i32> = 1..10001;

    pub async fn select_random_world(&self) -> World {
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
    
        self.statements
            .select_world_by_id
            .query_as()
            .bind(rng.gen_range(Self::ID_RANGE))
            .fetch_one(&self.pool)
            .await
            .expect("failed to fetch a world")
    }
    
    pub async fn select_all_fortunes(&self) -> Vec<Fortune> {
        self.statements
            .select_all_fortunes
            .query_as()
            .fetch_all(&self.pool)
            .await
            .expect("failed to fetch fortunes")
    }
    
    pub async fn select_n_random_worlds(&self, n: usize) -> Vec<World> {
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
        
        let selects = FuturesUnordered::new();
        for _ in 0..n {
            selects.push(
                self.statements
                    .select_world_by_id
                    .query_as()
                    .bind(rng.gen_range(Self::ID_RANGE))
                    .fetch_one(&self.pool)
            )
        }
        selects.try_collect().await.expect("failed to fetch worlds")
    }
    
    pub async fn update_randomnumbers_of_worlds(&self, n: usize) -> Vec<World> {
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();

        let mut tx = self.pool.begin().await.expect("failed to begin transaction");

        let mut worlds: Vec<World> = Vec::with_capacity(n);
        for _ in 0..n {
            worlds.push(
                self.statements
                    .select_world_by_id_for_update
                    .query_as()
                    .bind(rng.gen_range(Self::ID_RANGE))
                    .fetch_one(&mut *tx)
                    .await
                    .expect("failed to fetch world")
            );
        }

        let (mut ids, mut randomnumbers) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for w in &mut worlds {
            w.randomnumber = rng.gen_range(Self::ID_RANGE);
            ids.push(w.id);
            randomnumbers.push(w.randomnumber);
        }

        self.statements
            .update_worlds
            .query()
            .bind(ids)
            .bind(randomnumbers)
            .execute(&mut *tx)
            .await
            .expect("failed to update worlds");

        tx.commit().await.expect("failed to commit transaction");

        worlds
    }
}
