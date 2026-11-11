use criterion::{black_box, criterion_group, criterion_main, Criterion};
use midstream::{ReflexController, ReflexEvent, ReflexEventKind};

fn reflex_controller_benchmark(c: &mut Criterion) {
    c.bench_function("reflex_controller_1000_events", |b| {
        b.iter(|| {
            let mut controller = ReflexController::new(1024);
            for sequence in 1..=1000_u64 {
                let kind = match sequence % 10 {
                    0 => ReflexEventKind::Interrupt,
                    1 => ReflexEventKind::Backchannel,
                    2 => ReflexEventKind::Cancel,
                    _ => ReflexEventKind::Observation,
                };
                let receipt = controller.observe(
                    ReflexEvent {
                        sequence,
                        at_micros: sequence * 100,
                        kind,
                    },
                    4,
                );
                black_box(receipt);
                if controller.queued() >= 1000 {
                    black_box(controller.drain_for_reasoner());
                }
            }
            black_box(controller.drain_for_reasoner());
        });
    });
}

criterion_group!(benches, reflex_controller_benchmark);
criterion_main!(benches);
