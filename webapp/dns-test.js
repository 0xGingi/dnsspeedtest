const DNS_PROVIDERS = [
    { name: "Google", endpoint: "https://dns.google/resolve" },
    { name: "Cloudflare", endpoint: "https://cloudflare-dns.com/dns-query" },
    //{ name: "Quad9", endpoint: "https://dns.quad9.net/dns-query" },
    //{ name: "OpenDNS", endpoint: "https://doh.opendns.com/dns-query" },
    //{ name: "AdGuard", endpoint: "https://dns.adguard-dns.com/dns-query" },
    //{ name: "Mullvad", endpoint: "https://dns.mullvad.net/dns-query" },
    { name: "DNS0", endpoint: "https://zero.dns0.eu/dns-query" },
    { name: "NextDNS", endpoint: "https://dns.nextdns.io/" },
    //{ name: "ControlD", endpoint: "https://dns.controld.com/comss" }
];

const TEST_DOMAINS = [
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

const TEST_ROUNDS = 5;
const TIMEOUT_MS = 3000;
const COOLDOWN_MS = 100;

class TestResult {
    constructor(provider) {
        this.provider = provider;
        this.durations = [];
        this.failedDomains = [];
        this.totalQueries = 0;
    }

    get successRate() {
        return (this.durations.length / this.totalQueries) * 100;
    }

    get avgDuration() {
        if (this.durations.length === 0) return TIMEOUT_MS;
        return this.durations.reduce((a, b) => a + b) / this.durations.length;
    }

    get medianDuration() {
        if (this.durations.length === 0) return TIMEOUT_MS;
        const sorted = [...this.durations].sort((a, b) => a - b);
        return sorted[Math.floor(sorted.length / 2)];
    }

    get minLatency() {
        return this.durations.length ? Math.min(...this.durations) : TIMEOUT_MS;
    }

    get maxLatency() {
        return this.durations.length ? Math.max(...this.durations) : TIMEOUT_MS;
    }
}

async function measureDNSLatency(domain, provider) {
    const startTime = performance.now();
    try {
        let dnsQuery;
        if (provider.endpoint.includes('dns.google')) {
            dnsQuery = `${provider.endpoint}?name=${domain}&type=A`;
        } else if (provider.format === 'dns') {
            dnsQuery = `${provider.endpoint}?dns=${domain}&type=A`;
        } else if (provider.format === 'wire') {
            return null;
        } else {
            dnsQuery = `${provider.endpoint}?name=${domain}&type=A&ct=application/dns-json`;
        }

        const proxyUrl = `/proxy?url=${encodeURIComponent(dnsQuery)}`;
        console.log('Requesting:', proxyUrl);
        
        const response = await fetch(proxyUrl);
        
        if (!response.ok) {
            const errorData = await response.json();
            throw new Error(errorData.error || 'DNS query failed');
        }
        
        const data = await response.json();
        if (!data || data.error) {
            throw new Error(data.error || 'Invalid DNS response');
        }
        
        return performance.now() - startTime;
    } catch (error) {
        console.error(`Error querying ${domain} using ${provider.name}:`, error);
        return null;
    }
}

async function testDNSSpeed(provider, updateProgress) {
    const result = new TestResult(provider.name);

    await measureDNSLatency("example.com", provider.endpoint);
    await new Promise(resolve => setTimeout(resolve, COOLDOWN_MS));

    for (let round = 0; round < TEST_ROUNDS; round++) {
        for (const domain of TEST_DOMAINS) {
            result.totalQueries++;
            updateProgress(`Testing ${provider.name}: ${domain} (Round ${round + 1}/${TEST_ROUNDS})`);

            const duration = await measureDNSLatency(domain, provider.endpoint);
            if (duration === null) {
                result.failedDomains.push(domain);
            } else {
                result.durations.push(duration);
            }

            await new Promise(resolve => setTimeout(resolve, COOLDOWN_MS));
        }

        if (round < TEST_ROUNDS - 1) {
            await new Promise(resolve => setTimeout(resolve, COOLDOWN_MS * 2));
        }
    }

    return result;
}

function displayResults(results) {
    const resultsDiv = document.getElementById('results');
    const sortedResults = [...results].sort((a, b) => a.medianDuration - b.medianDuration);

    let html = `
        <h2>Detailed Results (sorted by median speed)</h2>
        <table>
            <tr>
                <th>Provider</th>
                <th>Median (ms)</th>
                <th>Avg (ms)</th>
                <th>Min (ms)</th>
                <th>Max (ms)</th>
                <th>Success Rate</th>
            </tr>
    `;

    sortedResults.forEach(result => {
        html += `
            <tr>
                <td>${result.provider}</td>
                <td>${result.medianDuration.toFixed(2)}</td>
                <td>${result.avgDuration.toFixed(2)}</td>
                <td>${result.minLatency.toFixed(2)}</td>
                <td>${result.maxLatency.toFixed(2)}</td>
                <td>${result.successRate.toFixed(1)}%</td>
            </tr>
            ${result.failedDomains.length ? `
            <tr>
                <td colspan="6" class="failed-domains">
                    Failed domains: ${result.failedDomains.join(', ')}
                </td>
            </tr>
            ` : ''}
        `;
    });

    html += '</table>';

    if (sortedResults.length > 0) {
        const fastest = sortedResults[0];
        html += `
            <h3>Fastest DNS provider: ${fastest.provider} 
            (${fastest.medianDuration.toFixed(2)}ms median, 
            ${fastest.successRate.toFixed(1)}% success rate)</h3>
        `;
    }

    resultsDiv.innerHTML = html;
}

document.getElementById('startTest').addEventListener('click', async () => {
    const button = document.getElementById('startTest');
    const progressDiv = document.getElementById('progress');
    button.disabled = true;
    const results = [];

    for (const provider of DNS_PROVIDERS) {
        const result = await testDNSSpeed(provider, (message) => {
            progressDiv.textContent = message;
        });
        results.push(result);
        displayResults(results);
    }

    progressDiv.textContent = 'Testing complete!';
    button.disabled = false;
});