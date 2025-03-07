use crate::models::{World, Fortune};
use std::sync::Arc;
use futures_util::stream::{StreamExt, FuturesUnordered};
use rand::{rngs::SmallRng, SeedableRng, Rng, distributions::Uniform, thread_rng};

#[derive(Clone)]
pub struct Postgres {
    pool: Arc<PostgresPool>,
}
impl Postgres {
    pub async fn new() -> Self {
        let pool = PostgresPool::new().await;

        Self { pool: Arc::new(pool) }
    }

    #[inline]
    pub async fn select_random_world(&self) -> World {
        self.pool.get().select_random_world().await
    }

    #[inline]
    pub async fn select_n_random_worlds(&self, n: usize) -> Vec<World> {
        self.pool.get().select_n_random_worlds(n).await
    }

    #[inline]
    pub async fn select_all_fortunes(&self) -> Vec<Fortune> {
        self.pool.get().select_all_fortunes().await
    }

    #[inline]
    pub async fn update_randomnumbers_of_n_worlds(&self, n: usize) -> Vec<World> {
        self.pool.get().update_randomnumbers_of_n_worlds(n).await
    }
}

struct PostgresPool {
    lock: std::sync::RwLock<()>,
    pending: std::cell::UnsafeCell<Vec<Client>>,
    known: std::cell::UnsafeCell<
        std::collections::HashMap<
            std::thread::ThreadId,
            Client,
            std::hash::BuildHasherDefault<AsIsHasher>
        >
    >,
}
unsafe impl Send for PostgresPool {}
unsafe impl Sync for PostgresPool {}
struct AsIsHasher(u64);
const _: () = {
    impl Default for AsIsHasher {
        fn default() -> Self {
            Self(0)
        }
    }

    impl std::hash::Hasher for AsIsHasher {
        fn write(&mut self, _bytes: &[u8]) {
            unreachable!()
        }
        fn write_u64(&mut self, i: u64) {
            self.0 = i;
        }
        fn finish(&self) -> u64 {
            self.0
        }
    }
};
impl PostgresPool {
    async fn new() -> Self {
        let size = num_cpus::get();

        let mut clients = Vec::with_capacity(size);
        for _ in 0..size {
            clients.push(Client::new().await);
        }

        let map = std::collections::HashMap::with_capacity_and_hasher(
            size,
            Default::default()
        );

        Self {
            // size,
            pending: std::cell::UnsafeCell::new(clients),
            known: std::cell::UnsafeCell::new(map),
            lock: std::sync::RwLock::new(()),
        }
    }

    #[inline]
    fn get(&self) -> &Client {
        let thread_id = std::thread::current().id();

        {let _read = self.lock.read().unwrap();
            match (unsafe {&*self.known.get()}).get(&thread_id) {
                Some(client) => client,
                None => {drop(_read);
                    {let _write = self.lock.write().unwrap();
                        let client = unsafe {&mut *self.pending.get()}
                            .pop()
                            .unwrap();
                        (unsafe {&mut *self.known.get()})
                            .insert(thread_id, client);
                        &(unsafe {&*self.known.get()})[&thread_id]
                    }
                }
            }
        }
    }
}

struct Client {
    client:     tokio_postgres::Client,
    statements: TechEmpowerStatements,
}

struct TechEmpowerStatements {
    select_world_by_id:  tokio_postgres::Statement,
    select_all_fortunes: tokio_postgres::Statement,
    update_worlds:       tokio_postgres::Statement,
}

impl Client {
    const ID_RANGE: std::ops::Range<i32> = 1..10001;

    async fn new() -> Self {
        let (client, connection) = tokio_postgres::connect(
            &std::env::var("DATABASE_URL").unwrap(),
            tokio_postgres::NoTls
        ).await.expect("failed to connect database");

        tokio::spawn(async {
            if let Err(e) = connection.await {
                eprintln!("error in database connection: {e}");
            }
        });
        
        let statements = TechEmpowerStatements {
            select_world_by_id: client
                .prepare("SELECT id, randomnumber FROM world WHERE id = $1 LIMIT 1")
                .await
                .unwrap(),
            select_all_fortunes: client
                .prepare("SELECT id, message FROM fortune")
                .await
                .unwrap(),
            update_worlds: client
                .prepare("\
                    UPDATE world SET randomnumber = new.randomnumber FROM ( \
                        SELECT * FROM UNNEST($1::int[], $2::int[]) AS v(id, randomnumber) \
                    ) AS new WHERE world.id = new.id \
                ")
                .await
                .unwrap(),
        };

        Self { client, statements }
    }
    
    async fn select_random_world_by_id(&self, id: i32) -> World {
        let row = self.client
            .query_one(&self.statements.select_world_by_id, &[&id])
            .await
            .expect("failed to fetch a world");

        World {
            id:           row.get(0),
            randomnumber: row.get(1),
        }
    }
}

impl Client {
    async fn select_random_world(&self) -> World {
        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
        self.select_random_world_by_id(rng.gen_range(Self::ID_RANGE)).await
    }
    
    async fn select_n_random_worlds(&self, n: usize) -> Vec<World> {
        let rng = SmallRng::from_rng(&mut thread_rng()).unwrap();

        let selects = FuturesUnordered::new();
        for id in rng.sample_iter(Uniform::new(Self::ID_RANGE.start, Self::ID_RANGE.end)).take(n) {
            selects.push(self.select_random_world_by_id(id))
        }

        selects.collect::<Vec<World>>().await
    }
    
    async fn select_all_fortunes(&self) -> Vec<Fortune> {
        let mut rows = std::pin::pin!(self
            .client
            .query_raw::<_, _, &[i32; 0]>(&self.statements.select_all_fortunes, &[])
            .await
            .expect("failed to fetch fortunes")
        );

        let mut fortunes = Vec::new();
        while let Some(row) = rows.next().await.transpose().unwrap() {
            fortunes.push(Fortune {
                id:      row.get(0),
                message: row.get(1),
            });
        }

        fortunes
    }
    
    async fn update_randomnumbers_of_n_worlds(&self, n: usize) -> Vec<World> {
        let rng = SmallRng::from_rng(&mut thread_rng()).unwrap();

        let mut worlds = self.select_n_random_worlds(n).await;

        let mut ids = Vec::with_capacity(n);
        let new_randomnumbers = rng
            .sample_iter(Uniform::new(Self::ID_RANGE.start, Self::ID_RANGE.end))
            .take(n)
            .collect::<Vec<_>>();
        for i in 0..n {
            worlds[i].randomnumber = new_randomnumbers[i];
            ids.push(worlds[i].id);
        }

        self.client
            .execute(&self.statements.update_worlds, &[&ids, &new_randomnumbers])
            .await
            .expect("failed to update worlds");

        worlds
    }
}
