use atomiq::{Atomic, Atomizable, Atomize};
use std::sync::atomic::Ordering;

pub struct ProgressWithStage<Stage, Progress> {
    pub stage: Stage,
    pub progress: Progress,
}

impl<Stage, Progress> ProgressWithStage<Stage, Progress> {
    pub fn new(stage: Stage, progress: Progress) -> Self {
        Self { stage, progress }
    }
}

pub struct AtomicProgressWithStage<Stage, AtomicProgress>
where
    Stage: Atomizable,
{
    stage: Atomic<Stage>,
    pub progress: AtomicProgress,
}

impl<Stage, Progress, AtomicProgress> From<ProgressWithStage<Stage, Progress>>
    for AtomicProgressWithStage<Stage, AtomicProgress>
where
    Stage: Atomizable,
    AtomicProgress: From<Progress>,
{
    fn from(progress_with_stage: ProgressWithStage<Stage, Progress>) -> Self {
        Self {
            stage: progress_with_stage.stage.atomize(),
            progress: AtomicProgress::from(progress_with_stage.progress),
        }
    }
}

impl<Stage, AtomicProgress> AtomicProgressWithStage<Stage, AtomicProgress>
where
    Stage: Atomizable,
{
    pub fn new(stage: Stage, progress: impl Into<AtomicProgress>) -> Self {
        Self {
            stage: stage.atomize(),
            progress: progress.into(),
        }
    }

    pub fn load_stage(&self) -> Stage {
        self.stage.load(Ordering::Relaxed)
    }

    pub fn store_stage(&self, value: Stage) {
        self.stage.store(value, Ordering::Relaxed)
    }
}
