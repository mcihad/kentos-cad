//! Arka plandaki işler: dışa aktarma ve uzamsal dizin.
//!
//! Vitrinde işler gerçekte dosya yazmaz; zamanlayıcıyla ilerleyen bir
//! benzetimdir. Aynı anda tek iş sürer, diğerleri sırasını bekler. DXF'e
//! dışa aktarma ilk denemede başarısız olur; hata durumu ve "Yeniden dene"
//! böyle görülür.

/// İşin türü.
#[derive(Debug, Clone, PartialEq)]
pub enum JobKind {
    /// Bütün katmanları verilen biçimde (PDF, PNG, DXF, GeoJSON) dosyaya yazar.
    Export(&'static str),
    /// Katmanların uzamsal dizinini yeniden kurar; süresi önceden bilinmez.
    Index,
}

/// İşin durumu.
#[derive(Debug, Clone, PartialEq)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed(String),
    Cancelled,
}

/// Arka plandaki bir iş.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    pub state: JobState,
    /// İşlenen ve toplam birim (öğe, pafta); dizinde adım.
    pub done: u32,
    pub total: u32,
    /// Bu birime gelince başarısız olur.
    fail_at: Option<u32>,
}

impl Job {
    pub fn title(&self) -> String {
        match &self.kind {
            JobKind::Export(format) => format!("{format} olarak dışa aktar"),
            JobKind::Index => "Uzamsal dizin oluştur".to_owned(),
        }
    }

    /// İşin ne yaptığı; bitince sonucu, başarısızsa nedeni.
    pub fn detail(&self) -> String {
        match (&self.kind, &self.state) {
            (_, JobState::Failed(reason)) => reason.clone(),
            (JobKind::Export(_), JobState::Cancelled) => {
                format!(
                    "{} / {} öğede durduruldu; dosya yazılmadı.",
                    self.done, self.total
                )
            }
            (JobKind::Index, JobState::Cancelled) => {
                "Durduruldu; önceki dizin kullanılıyor.".to_owned()
            }
            (JobKind::Export(format), JobState::Done) => {
                format!("{}: {} öğe yazıldı.", file_name(format), self.total)
            }
            (JobKind::Export(format), _) => {
                format!("{}: {} / {} öğe", file_name(format), self.done, self.total)
            }
            (JobKind::Index, JobState::Done) => "6 katman, 60 öğe dizinlendi.".to_owned(),
            (JobKind::Index, _) => "Katmanlar taranıyor".to_owned(),
        }
    }

    /// Oranı bilinen işte 0..1; dizin oluşturmada bilinmez.
    pub fn progress(&self) -> Option<f32> {
        match self.kind {
            JobKind::Index => None,
            _ => Some(self.done as f32 / self.total.max(1) as f32),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, JobState::Queued | JobState::Running)
    }

    /// Durum çubuğunda gösterilen kısa ad.
    pub fn activity(&self) -> &'static str {
        match self.kind {
            JobKind::Export(_) => "Dışa aktarılıyor",
            JobKind::Index => "Dizin oluşturuluyor",
        }
    }

    /// Bir adımda ilerlenen birim.
    fn step(&self) -> u32 {
        match self.kind {
            JobKind::Export(_) => 2,
            JobKind::Index => 1,
        }
    }
}

/// Dışa aktarılan dosyanın adı.
pub fn file_name(format: &str) -> String {
    let extension = match format {
        "GeoJSON" => "geojson".to_owned(),
        other => other.to_ascii_lowercase(),
    };

    format!("Türkiye.{extension}")
}

/// Bir adımdan sonra biten ya da başarısız olan iş.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Done(Job),
    Failed(Job),
}

/// İş kuyruğu.
#[derive(Debug, Clone, Default)]
pub struct Jobs {
    list: Vec<Job>,
    next: u64,
    /// DXF bir kez başarısız olur; yeniden denemede biter.
    dxf_failed: bool,
}

impl Jobs {
    /// İşi sıraya ekler.
    pub fn push(&mut self, kind: JobKind, total: u32) -> u64 {
        self.next += 1;

        let fail_at = match kind {
            JobKind::Export("DXF") if !self.dxf_failed => {
                self.dxf_failed = true;
                Some(total * 7 / 10)
            }
            _ => None,
        };

        self.list.push(Job {
            id: self.next,
            kind,
            state: JobState::Queued,
            done: 0,
            total,
            fail_at,
        });

        self.next
    }

    pub fn iter(&self) -> impl Iterator<Item = &Job> {
        self.list.iter()
    }

    /// Süren ya da sırada bekleyen iş var mı.
    pub fn is_busy(&self) -> bool {
        self.list.iter().any(Job::is_active)
    }

    pub fn running(&self) -> Option<&Job> {
        self.list.iter().find(|job| job.state == JobState::Running)
    }

    pub fn queued(&self) -> usize {
        self.list
            .iter()
            .filter(|job| job.state == JobState::Queued)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.list
            .iter()
            .filter(|job| matches!(job.state, JobState::Failed(_)))
            .count()
    }

    /// Bitmiş işler (başarılı, başarısız ya da iptal) var mı.
    pub fn has_finished(&self) -> bool {
        self.list.iter().any(|job| !job.is_active())
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Bir adım ilerler: süren iş yoksa sıradakini başlatır, süreni bir adım
    /// yürütür. Biten ya da başarısız olan işi döndürür.
    pub fn tick(&mut self) -> Option<Outcome> {
        if self.running().is_none()
            && let Some(job) = self
                .list
                .iter_mut()
                .find(|job| job.state == JobState::Queued)
        {
            job.state = JobState::Running;
        }

        let job = self
            .list
            .iter_mut()
            .find(|job| job.state == JobState::Running)?;

        job.done = (job.done + job.step()).min(job.total);

        if job.fail_at.is_some_and(|at| job.done >= at) {
            job.state = JobState::Failed(
                "Karayolları: çizgi tipi DXF'te karşılanamadı; dosya yarım kaldı.".to_owned(),
            );
            return Some(Outcome::Failed(job.clone()));
        }

        if job.done >= job.total {
            job.state = JobState::Done;
            return Some(Outcome::Done(job.clone()));
        }

        None
    }

    /// Süren ya da sıradaki işi durdurur.
    pub fn cancel(&mut self, id: u64) -> Option<&Job> {
        let job = self
            .list
            .iter_mut()
            .find(|job| job.id == id && job.is_active())?;

        job.state = JobState::Cancelled;
        Some(job)
    }

    /// Başarısız ya da iptal edilen işi baştan sıraya koyar.
    pub fn retry(&mut self, id: u64) -> bool {
        let Some(job) = self
            .list
            .iter_mut()
            .find(|job| job.id == id && !job.is_active())
        else {
            return false;
        };

        job.state = JobState::Queued;
        job.done = 0;
        job.fail_at = None;
        true
    }

    /// İşi listeden kaldırır; süren iş kaldırılmaz.
    pub fn dismiss(&mut self, id: u64) {
        self.list.retain(|job| job.id != id || job.is_active());
    }

    /// Biten işlerin hepsini kaldırır.
    pub fn clear_finished(&mut self) {
        self.list.retain(Job::is_active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_run_one_at_a_time() {
        let mut jobs = Jobs::default();

        let export = jobs.push(JobKind::Export("GeoJSON"), 4);
        let index = jobs.push(JobKind::Index, 3);

        assert_eq!(jobs.tick(), None);
        assert_eq!(jobs.running().map(|job| job.id), Some(export));
        assert_eq!(jobs.queued(), 1);

        assert!(matches!(jobs.tick(), Some(Outcome::Done(job)) if job.id == export));

        // Sıradaki iş kendiliğinden başlar.
        jobs.tick();
        assert_eq!(jobs.running().map(|job| job.id), Some(index));
        assert_eq!(jobs.running().and_then(Job::progress), None);
    }

    #[test]
    fn failed_jobs_can_be_retried_and_cancelled_jobs_stop() {
        let mut jobs = Jobs::default();
        let dxf = jobs.push(JobKind::Export("DXF"), 10);

        let outcome = (0..10).find_map(|_| jobs.tick());
        assert!(matches!(outcome, Some(Outcome::Failed(job)) if job.done == 8));

        // Yeniden denemede başarısız olmaz.
        assert!(jobs.retry(dxf));
        let outcome = (0..10).find_map(|_| jobs.tick());
        assert!(matches!(outcome, Some(Outcome::Done(_))));

        let png = jobs.push(JobKind::Export("PNG"), 10);
        jobs.tick();
        assert!(jobs.cancel(png).is_some());
        assert!(!jobs.is_busy());

        jobs.clear_finished();
        assert!(jobs.is_empty());
    }
}
