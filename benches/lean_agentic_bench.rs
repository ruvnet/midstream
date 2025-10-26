//! Comprehensive benchmarks for Lean Agentic Learning System
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use midstream::{
    LeanAgenticSystem, LeanAgenticConfig, AgentContext,
    FormalReasoner, AgenticLoop, KnowledgeGraph, StreamLearner,
    Action, Entity, EntityType,
    TemporalComparator, Sequence, ComparisonAlgorithm,
    RealtimeScheduler, SchedulingPolicy, Priority,
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

fn benchmark_temporal_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("temporal_comparison");

    // Benchmark DTW with different sequence sizes
    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("dtw", size),
            size,
            |b, &size| {
                let mut comparator = TemporalComparator::<i32>::new();
                let seq1: Vec<i32> = (0..size).collect();
                let seq2: Vec<i32> = (0..size).map(|x| x + (x % 3)).collect();

                b.iter(|| {
                    black_box(comparator.compare(&seq1, &seq2, ComparisonAlgorithm::DTW))
                });
            },
        );
    }

    // Benchmark LCS
    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("lcs", size),
            size,
            |b, &size| {
                let mut comparator = TemporalComparator::<i32>::new();
                let seq1: Vec<i32> = (0..size).collect();
                let seq2: Vec<i32> = (0..size).map(|x| x + (x % 2)).collect();

                b.iter(|| {
                    black_box(comparator.compare(&seq1, &seq2, ComparisonAlgorithm::LCS))
                });
            },
        );
    }

    // Benchmark edit distance
    group.bench_function("edit_distance", |b| {
        let mut comparator = TemporalComparator::<char>::new();
        let seq1: Vec<char> = "kitten".chars().collect();
        let seq2: Vec<char> = "sitting".chars().collect();

        b.iter(|| {
            black_box(comparator.compare(&seq1, &seq2, ComparisonAlgorithm::EditDistance))
        });
    });

    // Benchmark pattern detection
    group.bench_function("pattern_detection", |b| {
        let comparator = TemporalComparator::<i32>::new();
        let sequence: Vec<i32> = (0..1000).map(|x| x % 10).collect();
        let pattern = vec![1, 2, 3];

        b.iter(|| {
            black_box(comparator.detect_pattern(&sequence, &pattern))
        });
    });

    // Benchmark find similar with cache
    group.bench_function("find_similar", |b| {
        let mut comparator = TemporalComparator::<i32>::new();

        // Add many sequences
        for i in 0..100 {
            comparator.add_sequence(Sequence {
                data: (0..50).map(|x| x + i).collect(),
                timestamp: i as i64,
                id: format!("seq_{}", i),
            });
        }

        let query: Vec<i32> = (0..50).collect();

        b.iter(|| {
            black_box(comparator.find_similar(&query, 0.7, ComparisonAlgorithm::LCS))
        });
    });

    group.finish();
}

fn benchmark_scheduler(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler");

    let rt = Runtime::new().unwrap();

    // Benchmark task scheduling
    group.bench_function("schedule_task", |b| {
        b.iter(|| {
            rt.block_on(async {
                let scheduler = RealtimeScheduler::new(SchedulingPolicy::EarliestDeadlineFirst);
                let action = Action {
                    action_type: "test".to_string(),
                    description: "Test task".to_string(),
                    parameters: HashMap::new(),
                    tool_calls: vec![],
                    expected_outcome: None,
                    expected_reward: 0.8,
                };

                black_box(
                    scheduler.schedule(
                        action,
                        Priority::Medium,
                        std::time::Duration::from_secs(1),
                        std::time::Duration::from_millis(10),
                    ).await
                )
            })
        });
    });

    // Benchmark task retrieval with EDF
    group.bench_function("next_task_edf", |b| {
        b.iter(|| {
            rt.block_on(async {
                let scheduler = RealtimeScheduler::new(SchedulingPolicy::EarliestDeadlineFirst);

                // Schedule multiple tasks
                for i in 0..10 {
                    let action = Action {
                        action_type: format!("task_{}", i),
                        description: format!("Task {}", i),
                        parameters: HashMap::new(),
                        tool_calls: vec![],
                        expected_outcome: None,
                        expected_reward: 0.8,
                    };

                    scheduler.schedule(
                        action,
                        Priority::Medium,
                        std::time::Duration::from_millis(100 + i * 10),
                        std::time::Duration::from_millis(10),
                    ).await;
                }

                black_box(scheduler.next_task().await)
            })
        });
    });

    // Benchmark priority scheduling
    group.bench_function("next_task_priority", |b| {
        b.iter(|| {
            rt.block_on(async {
                let scheduler = RealtimeScheduler::new(SchedulingPolicy::FixedPriority);

                // Schedule tasks with different priorities
                for priority in &[Priority::Low, Priority::Medium, Priority::High, Priority::Critical] {
                    let action = Action {
                        action_type: format!("task_{:?}", priority),
                        description: format!("Task {:?}", priority),
                        parameters: HashMap::new(),
                        tool_calls: vec![],
                        expected_outcome: None,
                        expected_reward: 0.8,
                    };

                    scheduler.schedule(
                        action,
                        *priority,
                        std::time::Duration::from_secs(1),
                        std::time::Duration::from_millis(10),
                    ).await;
                }

                black_box(scheduler.next_task().await)
            })
        });
    });

    // Benchmark scheduling with high load
    for num_tasks in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("high_load", num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.iter(|| {
                    rt.block_on(async {
                        let scheduler = RealtimeScheduler::new(SchedulingPolicy::EarliestDeadlineFirst);

                        for i in 0..num_tasks {
                            let action = Action {
                                action_type: format!("task_{}", i),
                                description: format!("Task {}", i),
                                parameters: HashMap::new(),
                                tool_calls: vec![],
                                expected_outcome: None,
                                expected_reward: 0.8,
                            };

                            scheduler.schedule(
                                action,
                                Priority::Medium,
                                std::time::Duration::from_millis(100),
                                std::time::Duration::from_millis(10),
                            ).await;
                        }

                        // Retrieve all tasks
                        let mut count = 0;
                        while scheduler.next_task().await.is_some() {
                            count += 1;
                        }
                        black_box(count)
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
    benchmark_temporal_comparison,
    benchmark_scheduler,
);

criterion_main!(benches);
