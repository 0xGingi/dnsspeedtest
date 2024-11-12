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
    "netflix.com"
];

const TEST_ROUNDS: u32 = 3;

#[derive(Debug)]
struct TestResult {
    provider: String,
    avg_duration: Duration,
    min_latency: Duration,
    max_latency: Duration,
    success_rate: f64,
}

async fn measure_latency(addr: &str) -> Option<Duration> {
    let start = Instant::now();
    if let Ok(mut stream) = tokio::net::TcpStream::connect(format!("{}:53", addr)).await {
        let _ = stream.shutdown().await;
        Some(start.elapsed())
    } else {
        None
    }
}

async fn test_dns_speed(provider: &DnsProvider) -> TestResult {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(5);
    
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
    let mut total_duration = Duration::default();
    let mut successful_queries = 0;
    let mut total_queries = 0;
    let mut min_latency = Duration::from_secs(5);
    let mut max_latency = Duration::default();

    if let Some(latency) = measure_latency(provider.ip).await {
        min_latency = latency;
        max_latency = latency;
    }

    let _ = resolver.lookup_ip(Name::from_ascii("example.com").unwrap()).await;
    sleep(Duration::from_millis(100)).await;

    for _ in 0..TEST_ROUNDS {
        for domain in TEST_DOMAINS {
            total_queries += 1;
            let query_start = Instant::now();
            
            match resolver.lookup_ip(Name::from_ascii(domain).unwrap()).await {
                Ok(_) => {
                    let duration = query_start.elapsed();
                    total_duration += duration;
                    successful_queries += 1;
                    min_latency = min_latency.min(duration);
                    max_latency = max_latency.max(duration);
                },
                Err(_) => continue,
            }
            
            sleep(Duration::from_millis(50)).await;
        }
    }

    let success_rate = (successful_queries as f64) / (total_queries as f64) * 100.0;
    let avg_duration = if successful_queries > 0 {
        total_duration / successful_queries
    } else {
        Duration::from_secs(5)
    };

    TestResult {
        provider: provider.name.to_string(),
        avg_duration,
        min_latency,
        max_latency,
        success_rate,
    }
}

#[tokio::main]
async fn main() {
    println!("Testing DNS query speeds...\n");

    let mut results = Vec::new();
    
    for provider in DNS_PROVIDERS {
        print!("Testing {}... ", provider.name);
        let result = test_dns_speed(provider).await;
        println!("{:.2} ms (Success rate: {:.1}%)", 
            result.avg_duration.as_secs_f64() * 1000.0,
            result.success_rate
        );
        results.push(result);
    }

    results.sort_by(|a, b| a.avg_duration.cmp(&b.avg_duration));

    println!("\nDetailed Results (sorted by speed):");
    println!("{:-<75}", "");
    println!("{:<15} {:>10} {:>12} {:>12} {:>15}", 
        "Provider", "Avg (ms)", "Min (ms)", "Max (ms)", "Success Rate");
    println!("{:-<75}", "");
    
    for result in &results {
        println!(
            "{:<15} {:>10.2} {:>12.2} {:>12.2} {:>14.1}%",
            result.provider,
            result.avg_duration.as_secs_f64() * 1000.0,
            result.min_latency.as_secs_f64() * 1000.0,
            result.max_latency.as_secs_f64() * 1000.0,
            result.success_rate
        );
    }

    if let Some(fastest) = results.first() {
        println!("\nFastest DNS provider: {} ({:.2} ms, {:.1}% success rate)",
            fastest.provider,
            fastest.avg_duration.as_secs_f64() * 1000.0,
            fastest.success_rate
        );
    }
}