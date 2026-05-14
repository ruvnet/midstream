//! Benchmarks for the AIMDS analysis layer.
//!
//! Exercises the public API the crate exposes today
//! (`BehavioralAnalyzer`, `PolicyVerifier`, `LTLChecker`).
//!
//! The earlier revision referenced `aimds_core::{Action, State}`
//! (neither exists), `verify_policy(&action)` (the real signature
//! takes `&PromptInput`), `trace.add_state(State, props)` (the real
//! signature takes only `props`), and `AnalysisEngine::analyze_full`
//! (the engine exists but with a different surface). It also used
//! `b.to_async(&rt)` without enabling criterion's `async_tokio`
//! feature in the workspace deps. We rewrite the bench to match the
//! actual code and avoid optional criterion features.

use aimds_analysis::{
    BehavioralAnalyzer, LTLChecker, LTLFormula, PolicyVerifier, SecurityPolicy, Trace,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;

fn behavioral_analysis_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut group = c.benchmark_group("behavioral_analysis");

    for size in [100, 500, 1000].iter() {
        let analyzer = BehavioralAnalyzer::new(10).unwrap();
        let sequence: Vec<f64> = (0..*size).map(|i| (i as f64 * 0.1).sin()).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    analyzer
                        .analyze_behavior(black_box(&sequence))
                        .await
                        .unwrap()
                })
            });
        });
    }
    group.finish();
}

fn policy_verifier_setup_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_setup");

    for num_policies in [1, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_policies),
            num_policies,
            |b, n| {
                b.iter(|| {
                    let mut verifier = PolicyVerifier::new().unwrap();
                    for i in 0..*n {
                        let policy = SecurityPolicy::new(
                            format!("policy_{}", i),
                            format!("Test policy {}", i),
                            "G authenticated",
                        );
                        verifier.add_policy(policy);
                    }
                    verifier.policy_count()
                });
            },
        );
    }
    group.finish();
}

fn ltl_checking_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ltl_checking");

    for trace_len in [10, 50, 100].iter() {
        let checker = LTLChecker::new();
        let mut trace = Trace::new();

        for _ in 0..*trace_len {
            let mut props = HashMap::new();
            props.insert("authenticated".to_string(), true);
            trace.add_state(props);
        }

        let formula = LTLFormula::parse("G authenticated").unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(trace_len), trace_len, |b, _| {
            b.iter(|| checker.check_formula(black_box(&formula), black_box(&trace)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    behavioral_analysis_benchmark,
    policy_verifier_setup_benchmark,
    ltl_checking_benchmark
);
criterion_main!(benches);
