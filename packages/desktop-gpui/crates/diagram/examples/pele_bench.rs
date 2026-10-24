//! Measures Pele on QuickJS: runtime creation plus Pele load, then warm renders per sample.
//! Run in release, because QuickJS is C and the dev profile builds it at -O0:
//! `packages/desktop-gpui/cargo.sh run --release -p bachuc-diagram --example pele_bench`

use std::process::ExitCode;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, Context, Runtime};

const PELE: &str = include_str!("../assets/pele/pele.global.js");
const RENDERS: usize = 50;
const INIT_LIMIT_MS: f64 = 300.0;
const P95_LIMIT_MS: f64 = 16.0;

const FLOWCHART: &str = "flowchart TD\n  A[Start] --> B{Valid?}\n  B -->|yes| C[(Database)]\n  B -->|no| D[Reject]\n  C --> E[Done]\n  D --> E";
const SEQUENCE: &str = "sequenceDiagram\n  participant C as Client\n  participant D as Daemon\n  C->>D: hello\n  D-->>C: server_info\n  C->>D: fetch_agent_timeline\n  D-->>C: timeline page";
const CLASS: &str = "classDiagram\n  class Animal {\n    +String name\n    +eat()\n  }\n  class Dog {\n    +bark()\n  }\n  Animal <|-- Dog";
const STATE: &str = "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running: prompt\n  Running --> Idle: finished\n  Running --> Error: failed\n  Error --> [*]";
const ER: &str = "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE_ITEM : contains\n  CUSTOMER {\n    string name\n    string email\n  }";
const GANTT: &str = "gantt\n  title Plan\n  dateFormat YYYY-MM-DD\n  section Build\n  Spike :a1, 2026-10-25, 3d\n  Render :after a1, 5d\n  section Ship\n  Review :2026-11-02, 2d";
const PIE: &str = "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n  \"Rats\" : 15";
const XYCHART: &str = "xychart-beta\n  title \"Sales\"\n  x-axis [jan, feb, mar, apr, may]\n  y-axis \"Revenue\" 0 --> 100\n  bar [20, 45, 60, 35, 80]\n  line [25, 40, 55, 50, 75]";
const MINDMAP: &str =
    "mindmap\n  root((Plan))\n    Render\n      Pele\n      resvg\n    Cache\n    Tests";

fn big_flowchart() -> String {
    let mut src = String::from("flowchart LR\n");
    for i in 0..39 {
        src.push_str(&format!(
            "  N{i}[Node {i}] --> N{}[Node {}]\n",
            i + 1,
            i + 1
        ));
    }
    src
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let index = ((sorted.len() as f64 * p).ceil() as usize).clamp(1, sorted.len()) - 1;
    sorted[index]
}

fn main() -> ExitCode {
    let big = big_flowchart();
    let samples: Vec<(&str, &str)> = vec![
        ("flowchart", FLOWCHART),
        ("sequence", SEQUENCE),
        ("class", CLASS),
        ("state", STATE),
        ("er", ER),
        ("gantt", GANTT),
        ("pie", PIE),
        ("xychart", XYCHART),
        ("mindmap", MINDMAP),
        ("flowchart-40", &big),
    ];

    let started = Instant::now();
    let runtime = Runtime::new().expect("create runtime");
    let context = Context::full(&runtime).expect("create context");
    let loaded = context.with(|ctx| {
        ctx.eval::<(), _>(PELE)
            .catch(&ctx)
            .map_err(|e| e.to_string())
    });
    let init_ms = ms(started.elapsed());
    println!("init: {init_ms:.1} ms (runtime + context + load Pele)");
    if let Err(error) = loaded {
        println!("NO-GO: Pele failed to load: {error}");
        return ExitCode::FAILURE;
    }

    let mut failures = Vec::new();
    let mut all = Vec::new();
    println!(
        "{:<14} {:>9} {:>9} {:>9}",
        "sample", "p50 ms", "p95 ms", "svg bytes"
    );
    for (name, source) in &samples {
        let mut times = Vec::with_capacity(RENDERS);
        let mut svg_len = 0;
        for _ in 0..RENDERS {
            let start = Instant::now();
            let result = context.with(|ctx| {
                ctx.globals()
                    .set("__src", *source)
                    .catch(&ctx)
                    .map_err(|e| e.to_string())?;
                ctx.eval::<String, _>("pele.render(__src).svg")
                    .catch(&ctx)
                    .map_err(|e| e.to_string())
            });
            let elapsed = ms(start.elapsed());
            match result {
                Ok(svg) => {
                    svg_len = svg.len();
                    times.push(elapsed);
                }
                Err(error) => {
                    failures.push(format!("{name}: {error}"));
                    break;
                }
            }
        }
        if times.len() == RENDERS {
            all.extend_from_slice(&times);
            times.sort_by(f64::total_cmp);
            println!(
                "{name:<14} {:>9.2} {:>9.2} {svg_len:>9}",
                percentile(&times, 0.50),
                percentile(&times, 0.95)
            );
        } else {
            println!("{name:<14} FAILED");
        }
    }

    all.sort_by(f64::total_cmp);
    let overall_p95 = if all.is_empty() {
        f64::INFINITY
    } else {
        percentile(&all, 0.95)
    };
    println!(
        "overall p95: {overall_p95:.2} ms over {} renders",
        all.len()
    );

    if init_ms > INIT_LIMIT_MS {
        failures.push(format!("init {init_ms:.1} ms is over {INIT_LIMIT_MS} ms"));
    }
    if overall_p95 > P95_LIMIT_MS {
        failures.push(format!(
            "overall p95 {overall_p95:.2} ms is over {P95_LIMIT_MS} ms"
        ));
    }
    if failures.is_empty() {
        println!("GO");
        ExitCode::SUCCESS
    } else {
        println!("NO-GO: {}", failures.join("; "));
        ExitCode::FAILURE
    }
}
