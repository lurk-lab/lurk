use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

// A global benchmark tracker
static BENCHMARK: Lazy<Mutex<BenchmarkFramework>> =
    Lazy::new(|| Mutex::new(BenchmarkFramework::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingStats {
    pub total_time: Duration,
    pub count: usize,
    pub mean_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

impl TimingStats {
    fn new() -> Self {
        Self {
            total_time: Duration::new(0, 0),
            count: 0,
            mean_time: Duration::new(0, 0),
            min_time: Duration::MAX,
            max_time: Duration::new(0, 0),
        }
    }

    fn add_timing(&mut self, duration: Duration) {
        self.total_time += duration;
        self.count += 1;

        if duration < self.min_time {
            self.min_time = duration;
        }

        if duration > self.max_time {
            self.max_time = duration;
        }

        self.mean_time = self.total_time / self.count as u32;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientBenchmark {
    pub client_id: usize,
    pub operations: HashMap<String, TimingStats>,
}

impl ClientBenchmark {
    fn new(client_id: usize) -> Self {
        Self {
            client_id,
            operations: HashMap::new(),
        }
    }

    fn add_operation_time(&mut self, operation: &str, duration: Duration) {
        let stats = self
            .operations
            .entry(operation.to_string())
            .or_insert_with(TimingStats::new);
        stats.add_timing(duration);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub clients: Vec<ClientBenchmark>,
    pub total_clients: usize,
}

impl BenchmarkSummary {
    fn from_clients(clients: &[ClientBenchmark]) -> Self {
        let total_clients = clients.len();

        Self {
            clients: clients.to_vec(),
            total_clients,
        }
    }

    pub fn generate_markdown_table(&self) -> String {
        let mut markdown = String::new();

        // Table header
        markdown.push_str("| Operation | Total Time | Count | Mean Time | Min Time | Max Time |\n");
        markdown.push_str("|-----------|------------|-------|-----------|----------|----------|\n");

        // Table rows
        for client in &self.clients {
            let mut sorted_ops: Vec<(&String, &TimingStats)> = client.operations.iter().collect();
            sorted_ops.sort_by(|a, b| a.0.cmp(b.0));

            for (operation, stats) in sorted_ops {
                markdown.push_str(&format!(
                    "| {} | {:.2}s | {} | {:.2}s | {:.2}s | {:.2}s |\n",
                    operation,
                    stats.total_time.as_secs_f64(),
                    stats.count,
                    stats.mean_time.as_secs_f64(),
                    stats.min_time.as_secs_f64(),
                    stats.max_time.as_secs_f64()
                ));
            }
        }

        markdown
    }
}

#[derive(Debug)]
pub struct BenchmarkFramework {
    clients: HashMap<usize, ClientBenchmark>,
}

impl BenchmarkFramework {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn register_client(&mut self, client_id: usize) {
        self.clients
            .insert(client_id, ClientBenchmark::new(client_id));
    }

    pub fn add_operation_time(&mut self, client_id: usize, operation: &str, duration: Duration) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.add_operation_time(operation, duration);
        }
    }

    pub fn generate_summary(&self) -> BenchmarkSummary {
        let clients: Vec<ClientBenchmark> = self.clients.values().cloned().collect();
        BenchmarkSummary::from_clients(&clients)
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let summary = self.generate_summary();
        let json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(path, json)?;

        // Also save markdown report
        let markdown = summary.generate_markdown_table();
        let md_path = format!("{}.md", path.trim_end_matches(".json"));
        std::fs::write(md_path, markdown)?;

        Ok(())
    }
}

// Public API functions

pub async fn register_client(client_id: usize) {
    let mut framework = BENCHMARK.lock().await;
    framework.register_client(client_id);
}

pub async fn add_operation_time(client_id: usize, operation: &str, duration: Duration) {
    let mut framework = BENCHMARK.lock().await;
    framework.add_operation_time(client_id, operation, duration);
}

pub async fn save_benchmark_results(path: &str) -> std::io::Result<()> {
    let framework = BENCHMARK.lock().await;
    framework.save_to_file(path)
}

// Utility function to time an operation
pub async fn time_operation<F, T>(client_id: usize, operation_type: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();

    add_operation_time(client_id, operation_type, duration).await;

    result
}

// Utility function to time an operation
pub async fn time_async_operation<F, Fut, T>(client_id: usize, operation_type: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    println!("time_async_operation {operation_type}");
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();

    add_operation_time(client_id, operation_type, duration).await;

    result
}

// Example of how to integrate with your GQL client
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn benchmark_example() {
        // Register clients
        register_client(1).await;
        register_client(2).await;

        // Set lurk load times
        // set_lurk_load_time(1, Duration::from_secs(180)).await;
        // set_lurk_load_time(2, Duration::from_secs(185)).await;

        // Record some proof times
        add_operation_time(1, "proof", Duration::from_millis(500)).await;
        add_operation_time(1, "proof", Duration::from_millis(550)).await;
        add_operation_time(2, "proof", Duration::from_millis(525)).await;

        // Record verification times
        add_operation_time(1, "verification", Duration::from_millis(50)).await;
        add_operation_time(2, "verification", Duration::from_millis(55)).await;

        // Record data transfer times
        add_operation_time(1, "data transfer", Duration::from_millis(20)).await;
        add_operation_time(2, "data transfer", Duration::from_millis(22)).await;

        // Use the time_operation utility
        let result = time_async_operation(1, "proof", || async {
            sleep(Duration::from_millis(100)).await;
            "operation result"
        })
        .await;

        assert_eq!(result, "operation result");

        // Generate and print report
        // let report = get_markdown_report().await;
        // println!("{}", report);

        // Save results
        let _ = save_benchmark_results("benchmark_results.json").await;
    }
}
