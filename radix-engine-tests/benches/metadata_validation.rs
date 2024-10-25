use criterion::{criterion_group, criterion_main, Criterion};
use radix_common::prelude::*;
use radix_engine::object_modules::metadata::MetadataValueValidationError;
use scrypto::prelude::CheckedUrl;
use std::hint::black_box;

#[allow(unused_must_use)]
fn bench_validate_urls(c: &mut Criterion) {
    c.bench_function("metadata_validation::validate_urls", |b| {
        b.iter(|| {
            black_box(
                CheckedUrl::of("https://www.example.com/test?q=x")
                    .ok_or(MetadataValueValidationError::InvalidURL("".to_owned())),
            );
        })
    });
}

criterion_group!(metadata_validation, bench_validate_urls);
criterion_main!(metadata_validation);
