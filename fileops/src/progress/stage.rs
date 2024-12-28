use atomiq::compat::SimpleAtomic;
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

pub struct AtomicProgressWithStage<Stage, AtomicStage, AtomicProgress>
where
    AtomicStage: SimpleAtomic<Value = Stage>,
{
    stage: AtomicStage,
    pub progress: AtomicProgress,
}

impl<Stage, AtomicStage, Progress, AtomicProgress> From<ProgressWithStage<Stage, Progress>>
    for AtomicProgressWithStage<Stage, AtomicStage, AtomicProgress>
where
    AtomicStage: SimpleAtomic<Value = Stage>,
    AtomicProgress: From<Progress>,
{
    fn from(progress_with_stage: ProgressWithStage<Stage, Progress>) -> Self {
        Self {
            stage: AtomicStage::from(progress_with_stage.stage),
            progress: AtomicProgress::from(progress_with_stage.progress),
        }
    }
}

impl<Stage, AtomicStage, AtomicProgress> AtomicProgressWithStage<Stage, AtomicStage, AtomicProgress>
where
    AtomicStage: SimpleAtomic<Value = Stage>,
{
    pub fn new(stage: AtomicStage, progress: AtomicProgress) -> Self {
        Self { stage, progress }
    }

    pub fn load_stage(&self, ordering: Ordering) -> Stage {
        self.stage.load(ordering)
    }

    pub fn store_stage(&self, value: Stage, ordering: Ordering) {
        self.stage.store(value, ordering)
    }
}
