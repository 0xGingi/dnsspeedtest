use std::time::{Instant, Duration};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::Protocol;
use std::net::SocketAddr;
use tokio;
use tokio::time::sleep;
use tokio::io::AsyncWriteExt;
use hickory_resolver::Name;

struct DnsProvider {
    name: &'static str,
    ip: &'static str,
}

const DNS_PROVIDERS: &[DnsProvider] = &[
    DnsProvider { name: "Google", ip: "8.8.8.8" },
    DnsProvider { name: "Cloudflare", ip: "1.1.1.1" },
    DnsProvider { name: "Quad9", ip: "9.9.9.9" },
    DnsProvider { name: "OpenDNS", ip: "208.67.222.222" },
    DnsProvider { name: "AdGuard", ip: "94.140.14.14" },
    DnsProvider { name: "Mullvad", ip: "194.242.2.2" },
    DnsProvider { name: "DNS0", ip: "193.110.81.0" },
    DnsProvider { name: "NextDNS", ip: "45.90.28.0" },
    DnsProvider { name: "ControlD", ip: "76.76.2.0" },
];

const TEST_DOMAINS: &[&str] = &[
    "google.com",
    "gitlab.com", 
    "cloudflare.com",
    "microsoft.com",
    "github.com",
    "netflix.com",
    "amazon.com",
    "facebook.com",
    "wikipedia.org",
    "reddit.com"
];

const TEST_ROUNDS: u32 = 5;
const TIMEOUT_SECS: u64 = 3;
const COOLDOWN_MS: u64 = 100;

#[derive(Debug)]
struct TestResult {
    provider: String,
    avg_duration: Duration,
    min_latency: Duration,
    max_latency: Duration,
    success_rate: f64,
    failed_domains: Vec<String>,
    median_duration: Duration,
}

async fn measure_latency(addr: &str) -> Option<Duration> {
    let start = Instant::now();
    match tokio::time::timeout(
        Duration::from_secs(TIMEOUT_SECS),
        tokio::net::TcpStream::connect(format!("{}:53", addr))
    ).await {
        Ok(Ok(mut stream)) => {
            let _ = stream.shutdown().await;
            Some(start.elapsed())
        },
        _ => None
    }
}

async fn test_dns_speed(provider: &DnsProvider) -> TestResult {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(TIMEOUT_SECS);
    opts.attempts = 1;
    opts.use_hosts_file = false;
    opts.cache_size = 0;
    opts.edns0 = false;
    
    let socket_addr = format!("{}:53", provider.ip)
        .parse::<SocketAddr>()
        .unwrap();
    
    let config = ResolverConfig::from_parts(
        None,
        vec![],
        vec![hickory_resolver::config::NameServerConfig::new(
            socket_addr,
            Protocol::Udp
        )],
    );

    let resolver = TokioAsyncResolver::tokio(config, opts);
    let mut durations = Vec::new();
    let mut failed_domains = Vec::new();
    let mut total_queries = 0;

    let _ = resolver.lookup_ip(Name::from_ascii("example.com").unwrap()).await;
    sleep(Duration::from_millis(COOLDOWN_MS)).await;

    for round in 0..TEST_ROUNDS {
        for domain in TEST_DOMAINS {
            total_queries += 1;
            
            let tcp_latency = measure_latency(provider.ip).await;
            if tcp_latency.is_none() {
                failed_domains.push(format!("{} (TCP Failed)", domain));
                continue;
            }
            
            let query_start = Instant::now();
            match resolver.lookup_ip(Name::from_ascii(domain).unwrap()).await {
                Ok(_) => {
                    durations.push(query_start.elapsed());
                },
                Err(_) => {
                    failed_domains.push(domain.to_string());
                }
            }
            
            sleep(Duration::from_millis(COOLDOWN_MS)).await;
        }

        if round < TEST_ROUNDS - 1 {
            sleep(Duration::from_millis(COOLDOWN_MS * 2)).await;
        }
    }

    durations.sort();
    let successful_queries = durations.len();
    let success_rate = (successful_queries as f64) / (total_queries as f64) * 100.0;

    let avg_duration = if !durations.is_empty() {
        Duration::from_secs_f64(
            durations.iter().map(|d| d.as_secs_f64()).sum::<f64>() / successful_queries as f64
        )
    } else {
        Duration::from_secs(TIMEOUT_SECS)
    };

    let min_latency = durations.first().copied().unwrap_or(Duration::from_secs(TIMEOUT_SECS));
    let max_latency = durations.last().copied().unwrap_or(Duration::from_secs(TIMEOUT_SECS));
    let median_duration = if !durations.is_empty() {
        durations[durations.len() / 2]
    } else {
        Duration::from_secs(TIMEOUT_SECS)
    };

    TestResult {
        provider: provider.name.to_string(),
        avg_duration,
        min_latency,
        max_latency,
        success_rate,
        failed_domains,
        median_duration,
    }
}

#[tokio::main]
async fn main() {
    println!("DNS Speed Test (Testing {} domains × {} rounds)\n", TEST_DOMAINS.len(), TEST_ROUNDS);

    let mut results = Vec::new();
    
    for provider in DNS_PROVIDERS {
        print!("Testing {}... ", provider.name);
        let result = test_dns_speed(provider).await;
        println!("{:.2} ms (Success rate: {:.1}%)", 
            result.median_duration.as_secs_f64() * 1000.0,
            result.success_rate
        );
        results.push(result);
    }

    results.sort_by(|a, b| a.median_duration.cmp(&b.median_duration));

    println!("\nDetailed Results (sorted by median speed):");
    println!("{:-<90}", "");
    println!("{:<15} {:>10} {:>10} {:>12} {:>12} {:>15}", 
        "Provider", "Median", "Avg (ms)", "Min (ms)", "Max (ms)", "Success Rate");
    println!("{:-<90}", "");
    
    for result in &results {
        println!(
            "{:<15} {:>10.2} {:>10.2} {:>12.2} {:>12.2} {:>14.1}%",
            result.provider,
            result.median_duration.as_secs_f64() * 1000.0,
            result.avg_duration.as_secs_f64() * 1000.0,
            result.min_latency.as_secs_f64() * 1000.0,
            result.max_latency.as_secs_f64() * 1000.0,
            result.success_rate
        );

        if !result.failed_domains.is_empty() {
            println!("    Failed domains: {}", result.failed_domains.join(", "));
        }
    }

    if let Some(fastest) = results.first() {
        println!("\nFastest DNS provider: {} ({:.2} ms median, {:.1}% success rate)",
            fastest.provider,
            fastest.median_duration.as_secs_f64() * 1000.0,
            fastest.success_rate
        );
    }

    println!("\nPress Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}