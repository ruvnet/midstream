//! Comprehensive benchmarks for Lean Agentic Learning System
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use midstream::{
    LeanAgenticSystem, LeanAgenticConfig, AgentContext,
    FormalReasoner, AgenticLoop, KnowledgeGraph, StreamLearner,
    Action, Entity, EntityType,
};
use std::collections::HashMap;
use tokio::runtime::Runtime;

fn benchmark_formal_reasoning(c: &mut Criterion) {
    let mut group = c.benchmark_group("formal_reasoning");

    let rt = Runtime::new().unwrap();

    // Benchmark action verification
    group.bench_function("verify_action", |b| {
        b.iter(|| {
            rt.block_on(async {
                let reasoner = FormalReasoner::new();
                let action = Action {
                    action_type: "test".to_string(),
                    description: "Test action".to_string(),
                    parameters: HashMap::new(),
                    tool_calls: vec![],
                    expected_outcome: Some("success".to_string()),
                    expected_reward: 0.8,
                };
                let context = AgentContext::new("test_session".to_string());

                black_box(reasoner.verify_action(&action, &context).await.unwrap())
            })
        });
    });

    // Benchmark theorem proving
    group.bench_function("prove_theorem", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut reasoner = FormalReasoner::new();
                black_box(
                    reasoner.prove_theorem(
                        "Q".to_string(),
                        vec!["P".to_string(), "P -> Q".to_string()],
                    ).await
                )
            })
        });
    });

    group.finish();
}

fn benchmark_agentic_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("agentic_loop");

    let rt = Runtime::new().unwrap();
    let config = LeanAgenticConfig::default();

    // Benchmark planning phase
    group.bench_function("plan", |b| {
        b.iter(|| {
            rt.block_on(async {
                let agent = AgenticLoop::new(config.clone());
                let context = AgentContext::new("test_session".to_string());

                black_box(agent.plan(&context, "What is the weather?").await.unwrap())
            })
        });
    });

    // Benchmark action selection
    group.bench_function("select_and_execute", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut agent = AgenticLoop::new(config.clone());
                let context = AgentContext::new("test_session".to_string());
                let plan = agent.plan(&context, "test input").await.unwrap();
                let action = agent.select_action(&plan).await.unwrap();

                black_box(agent.execute(&action).await.unwrap())
            })
        });
    });

    // Benchmark learning update
    group.bench_function("learn", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut agent = AgenticLoop::new(config.clone());
                let context = AgentContext::new("test_session".to_string());
                let plan = agent.plan(&context, "test").await.unwrap();
                let action = agent.select_action(&plan).await.unwrap();
                let observation = agent.execute(&action).await.unwrap();
                let reward = agent.compute_reward(&observation).await.unwrap();

                let signal = midstream::LearningSignal {
                    action,
                    observation,
                    reward,
                };

                black_box(agent.learn(signal).await.unwrap())
            })
        });
    });

    group.finish();
}

fn benchmark_knowledge_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("knowledge_graph");

    let rt = Runtime::new().unwrap();

    // Benchmark entity extraction
    group.bench_function("extract_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let kg = KnowledgeGraph::new();
                let text = "Alice works at Google in California with Bob and Charlie";

                black_box(kg.extract_entities(text).await.unwrap())
            })
        });
    });

    // Benchmark graph updates
    group.bench_function("update_graph", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut kg = KnowledgeGraph::new();
                let entities = vec![
                    Entity {
                        id: "e1".to_string(),
                        name: "Alice".to_string(),
                        entity_type: EntityType::Person,
                        attributes: HashMap::new(),
                        confidence: 0.9,
                    },
                    Entity {
                        id: "e2".to_string(),
                        name: "Google".to_string(),
                        entity_type: EntityType::Organization,
                        attributes: HashMap::new(),
                        confidence: 0.95,
                    },
                ];

                black_box(kg.update(entities).await.unwrap())
            })
        });
    });

    // Benchmark relation finding
    group.bench_function("find_related", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut kg = KnowledgeGraph::new();

                // Setup
                kg.add_relation(midstream::Relation {
                    id: "r1".to_string(),
                    subject: "alice".to_string(),
                    predicate: "works_at".to_string(),
                    object: "google".to_string(),
                    confidence: 0.9,
                    source: "text".to_string(),
                });

                black_box(kg.find_related("alice", 2))
            })
        });
    });

    group.finish();
}

fn benchmark_stream_learning(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_learning");

    let rt = Runtime::new().unwrap();

    // Benchmark online learning update
    group.bench_function("online_update", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut learner = StreamLearner::new(0.01);
                let action = Action {
                    action_type: "test".to_string(),
                    description: "Test action".to_string(),
                    parameters: HashMap::new(),
                    tool_calls: vec![],
                    expected_outcome: None,
                    expected_reward: 0.5,
                };

                black_box(learner.update(&action, 1.0, "test context").await.unwrap())
            })
        });
    });

    // Benchmark reward prediction
    group.bench_function("predict_reward", |b| {
        b.iter(|| {
            rt.block_on(async {
                let learner = StreamLearner::new(0.01);
                let action = Action {
                    action_type: "test".to_string(),
                    description: "Test action".to_string(),
                    parameters: HashMap::new(),
                    tool_calls: vec![],
                    expected_outcome: None,
                    expected_reward: 0.5,
                };

                black_box(learner.predict_reward(&action, "test context").await)
            })
        });
    });

    group.finish();
}

fn benchmark_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    group.sample_size(50); // Reduce sample size for slower benchmarks

    let rt = Runtime::new().unwrap();

    // Benchmark full processing pipeline
    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let config = LeanAgenticConfig::default();
                    let system = LeanAgenticSystem::new(config);
                    let mut context = AgentContext::new("bench_session".to_string());

                    for i in 0..size {
                        let chunk = format!("Message {} with some content", i);
                        black_box(
                            system.process_stream_chunk(&chunk, context.clone()).await.unwrap()
                        );
                        context.add_message(chunk);
                    }
                })
            });
        });
    }

    group.finish();
}

fn benchmark_concurrent_sessions(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_sessions");
    group.sample_size(20);

    let rt = Runtime::new().unwrap();

    // Benchmark multiple concurrent sessions
    for num_sessions in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_sessions),
            num_sessions,
            |b, &num_sessions| {
                b.iter(|| {
                    rt.block_on(async {
                        let config = LeanAgenticConfig::default();
                        let system = LeanAgenticSystem::new(config);

                        let mut handles = vec![];

                        for i in 0..num_sessions {
                            let sys = &system;
                            let handle = tokio::spawn(async move {
                                let context = AgentContext::new(format!("session_{}", i));
                                sys.process_stream_chunk("test message", context)
                                    .await
                                    .unwrap()
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            black_box(handle.await.unwrap());
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_formal_reasoning,
    benchmark_agentic_loop,
    benchmark_knowledge_graph,
    benchmark_stream_learning,
    benchmark_end_to_end,
    benchmark_concurrent_sessions,
);

criterion_main!(benches);
