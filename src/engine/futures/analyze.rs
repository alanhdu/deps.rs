use failure::Error;
use futures::stream::futures_unordered;
use futures::{Future, Poll, Stream};

use crate::models::crates::{AnalyzedDependencies, CrateDeps};

use super::super::machines::analyzer::DependencyAnalyzer;
use super::super::Engine;

pub struct AnalyzeDependenciesFuture {
    inner: Box<dyn Future<Item = AnalyzedDependencies, Error = Error> + Send>,
}

impl AnalyzeDependenciesFuture {
    pub fn new(engine: Engine, deps: CrateDeps) -> Self {
        let future =
            engine.fetch_advisory_db().and_then(move |advisory_db| {
                let analyzer = DependencyAnalyzer::new(&deps, Some(advisory_db));

                let main_deps = deps.main.into_iter().filter_map(|(name, dep)| {
                    if dep.is_external() {
                        Some(name)
                    } else {
                        None
                    }
                });
                let dev_deps = deps.dev.into_iter().filter_map(|(name, dep)| {
                    if dep.is_external() {
                        Some(name)
                    } else {
                        None
                    }
                });
                let build_deps = deps.build.into_iter().filter_map(|(name, dep)| {
                    if dep.is_external() {
                        Some(name)
                    } else {
                        None
                    }
                });

                let release_futures =
                    engine.fetch_releases(main_deps.chain(dev_deps).chain(build_deps));

                futures_unordered(release_futures)
                    .fold(analyzer, |mut analyzer, releases| {
                        analyzer.process(releases);
                        Ok(analyzer) as Result<_, Error>
                    })
                    .map(|analyzer| analyzer.finalize())
            });

        AnalyzeDependenciesFuture {
            inner: Box::new(future),
        }
    }
}

impl Future for AnalyzeDependenciesFuture {
    type Item = AnalyzedDependencies;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}
