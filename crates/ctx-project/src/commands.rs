//! Comandos desfazíveis sobre o agregado [`Project`].

use crate::domain::*;
use shared_kernel::command::{Command, CommandError};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::tempo::{MeterEvent, TempoEvent};
use shared_kernel::{Db, FadeShape, Tick};

type R = Result<(), CommandError>;

fn missing_state() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

pub struct RenameProject {
    pub name: String,
    previous: Option<String>,
}
impl RenameProject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            previous: None,
        }
    }
}
impl Command<Project> for RenameProject {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(p.rename(&self.name)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.rename(self.previous.as_deref().ok_or_else(missing_state)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Renomear projeto"
    }
}

/// Cria uma faixa (a faixa é construída antes para que o id seja estável em refazer).
pub struct CreateTrack {
    track: Track,
    index: usize,
}
impl CreateTrack {
    pub fn new(track: Track, index: usize) -> Self {
        Self { track, index }
    }
    pub fn track_id(&self) -> TrackId {
        self.track.id
    }
}
impl Command<Project> for CreateTrack {
    fn apply(&mut self, p: &mut Project) -> R {
        self.index = p.insert_track(self.track.clone(), self.index)?;
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.remove_track(self.track.id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Criar faixa"
    }
}

pub struct DeleteTrack {
    pub id: TrackId,
    removed: Option<(Track, usize)>,
}
impl DeleteTrack {
    pub fn new(id: TrackId) -> Self {
        Self { id, removed: None }
    }
}
impl Command<Project> for DeleteTrack {
    fn apply(&mut self, p: &mut Project) -> R {
        self.removed = Some(p.remove_track(self.id)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let (t, i) = self.removed.take().ok_or_else(missing_state)?;
        p.insert_track(t, i)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover faixa"
    }
}

pub struct RenameTrack {
    pub id: TrackId,
    pub name: String,
    previous: Option<String>,
}
impl RenameTrack {
    pub fn new(id: TrackId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            previous: None,
        }
    }
}
impl Command<Project> for RenameTrack {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(p.rename_track(self.id, &self.name)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.rename_track(self.id, self.previous.as_deref().ok_or_else(missing_state)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Renomear faixa"
    }
}

pub struct MoveTrack {
    pub id: TrackId,
    pub to: usize,
    from: Option<usize>,
}
impl MoveTrack {
    pub fn new(id: TrackId, to: usize) -> Self {
        Self { id, to, from: None }
    }
}
impl Command<Project> for MoveTrack {
    fn apply(&mut self, p: &mut Project) -> R {
        self.from = Some(p.move_track(self.id, self.to)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.move_track(self.id, self.from.ok_or_else(missing_state)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Mover faixa"
    }
}

/// Registra uma fonte de áudio. Ao desfazer, a fonte sai do projeto (o arquivo permanece em disco
/// para que refazer funcione; a limpeza de órfãos ocorre ao salvar).
pub struct AddSource {
    source: AudioSource,
}
impl AddSource {
    pub fn new(source: AudioSource) -> Self {
        Self { source }
    }
    pub fn source_id(&self) -> AudioSourceId {
        self.source.id
    }
}
impl Command<Project> for AddSource {
    fn apply(&mut self, p: &mut Project) -> R {
        p.add_source(self.source.clone())?;
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.remove_source(self.source.id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Importar áudio"
    }
}

pub struct AddClip {
    track: TrackId,
    clip: Clip,
}
impl AddClip {
    pub fn new(track: TrackId, clip: Clip) -> Self {
        Self { track, clip }
    }
    pub fn clip_id(&self) -> ClipId {
        self.clip.id
    }
}
impl Command<Project> for AddClip {
    fn apply(&mut self, p: &mut Project) -> R {
        p.insert_clip(self.track, self.clip.clone(), usize::MAX)?;
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.remove_clip(self.clip.id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Adicionar clipe"
    }
}

pub struct DeleteClip {
    pub id: ClipId,
    removed: Option<(TrackId, Clip, usize)>,
}
impl DeleteClip {
    pub fn new(id: ClipId) -> Self {
        Self { id, removed: None }
    }
}
impl Command<Project> for DeleteClip {
    fn apply(&mut self, p: &mut Project) -> R {
        self.removed = Some(p.remove_clip(self.id)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let (t, c, i) = self.removed.take().ok_or_else(missing_state)?;
        p.place_clip(t, c, i)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover clipe"
    }
}

pub struct MoveClip {
    pub id: ClipId,
    pub to_track: TrackId,
    pub start: Tick,
    previous: Option<(TrackId, Tick, usize)>,
}
impl MoveClip {
    pub fn new(id: ClipId, to_track: TrackId, start: Tick) -> Self {
        Self {
            id,
            to_track,
            start,
            previous: None,
        }
    }
}
impl Command<Project> for MoveClip {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(p.move_clip(self.id, self.to_track, self.start)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let (track, start, index) = self.previous.ok_or_else(missing_state)?;
        let (_, mut clip, _) = p.remove_clip(self.id)?;
        clip.start = start;
        p.place_clip(track, clip, index)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Mover clipe"
    }
}

pub struct SetClipGain {
    pub id: ClipId,
    pub gain: Db,
    previous: Option<Db>,
}
impl SetClipGain {
    pub fn new(id: ClipId, gain: Db) -> Self {
        Self {
            id,
            gain,
            previous: None,
        }
    }
}
impl Command<Project> for SetClipGain {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(p.set_clip_gain(self.id, self.gain)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.set_clip_gain(self.id, self.previous.ok_or_else(missing_state)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Ganho do clipe"
    }
}

/// Substitui um clipe por uma versão editada calculada a partir do estado atual. Base comum
/// dos comandos de edição de um clipe só (cortar, fades, polaridade, reverso).
struct EditOne {
    id: ClipId,
    previous: Option<Clip>,
}
impl EditOne {
    fn new(id: ClipId) -> Self {
        Self { id, previous: None }
    }
    fn apply(
        &mut self,
        p: &mut Project,
        f: impl FnOnce(&Project, &Clip) -> Result<Clip, ProjectError>,
    ) -> R {
        let next = f(p, p.clip(self.id)?)?;
        self.previous = Some(p.replace_clip(next)?);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        p.replace_clip(self.previous.take().ok_or_else(missing_state)?)?;
        Ok(())
    }
}

/// Corta um clipe pelas bordas (RF-3.1): novo início na timeline e nova duração.
pub struct TrimClip {
    pub start: Tick,
    pub length_frames: u64,
    inner: EditOne,
}
impl TrimClip {
    pub fn new(id: ClipId, start: Tick, length_frames: u64) -> Self {
        Self {
            start,
            length_frames,
            inner: EditOne::new(id),
        }
    }
}
impl Command<Project> for TrimClip {
    fn apply(&mut self, p: &mut Project) -> R {
        let (start, len) = (self.start, self.length_frames);
        self.inner.apply(p, |p, c| p.trimmed_clip(c.id, start, len))
    }
    fn revert(&mut self, p: &mut Project) -> R {
        self.inner.revert(p)
    }
    fn label(&self) -> &str {
        "Cortar clipe"
    }
}

/// Define os fades de entrada e saída (RF-3.2).
pub struct SetClipFades {
    pub fade_in_frames: u64,
    pub fade_out_frames: u64,
    pub fade_in_shape: FadeShape,
    pub fade_out_shape: FadeShape,
    inner: EditOne,
}
impl SetClipFades {
    pub fn new(
        id: ClipId,
        fade_in_frames: u64,
        fade_out_frames: u64,
        fade_in_shape: FadeShape,
        fade_out_shape: FadeShape,
    ) -> Self {
        Self {
            fade_in_frames,
            fade_out_frames,
            fade_in_shape,
            fade_out_shape,
            inner: EditOne::new(id),
        }
    }
}
impl Command<Project> for SetClipFades {
    fn apply(&mut self, p: &mut Project) -> R {
        let (fi, fo, si, so) = (
            self.fade_in_frames,
            self.fade_out_frames,
            self.fade_in_shape,
            self.fade_out_shape,
        );
        self.inner.apply(p, |_, c| {
            let mut n = c.clone();
            n.edit.fade_in_frames = fi;
            n.edit.fade_out_frames = fo;
            n.edit.fade_in_shape = si;
            n.edit.fade_out_shape = so;
            Ok(n)
        })
    }
    fn revert(&mut self, p: &mut Project) -> R {
        self.inner.revert(p)
    }
    fn label(&self) -> &str {
        "Fades do clipe"
    }
}

/// Inverte (ou restaura) a polaridade do clipe (RF-3.3).
pub struct SetClipPolarity {
    pub inverted: bool,
    inner: EditOne,
}
impl SetClipPolarity {
    pub fn new(id: ClipId, inverted: bool) -> Self {
        Self {
            inverted,
            inner: EditOne::new(id),
        }
    }
}
impl Command<Project> for SetClipPolarity {
    fn apply(&mut self, p: &mut Project) -> R {
        let v = self.inverted;
        self.inner.apply(p, |_, c| {
            let mut n = c.clone();
            n.edit.polarity_inverted = v;
            Ok(n)
        })
    }
    fn revert(&mut self, p: &mut Project) -> R {
        self.inner.revert(p)
    }
    fn label(&self) -> &str {
        "Inverter fase"
    }
}

/// Reproduz o clipe de trás para frente (ou volta ao normal) (RF-3.3).
pub struct SetClipReversed {
    pub reversed: bool,
    inner: EditOne,
}
impl SetClipReversed {
    pub fn new(id: ClipId, reversed: bool) -> Self {
        Self {
            reversed,
            inner: EditOne::new(id),
        }
    }
}
impl Command<Project> for SetClipReversed {
    fn apply(&mut self, p: &mut Project) -> R {
        let v = self.reversed;
        self.inner.apply(p, |_, c| {
            let mut n = c.clone();
            n.edit.reversed = v;
            Ok(n)
        })
    }
    fn revert(&mut self, p: &mut Project) -> R {
        self.inner.revert(p)
    }
    fn label(&self) -> &str {
        "Reverter clipe"
    }
}

/// Divide um clipe no tick `at` (RF-3.1). A segunda parte recebe um id criado antes, para que
/// refazer produza o mesmo estado.
pub struct SplitClip {
    pub id: ClipId,
    pub at: Tick,
    new_id: ClipId,
    original: Option<Clip>,
}
impl SplitClip {
    pub fn new(id: ClipId, at: Tick) -> Self {
        Self {
            id,
            at,
            new_id: ClipId::new(),
            original: None,
        }
    }
    /// Id da segunda parte.
    pub fn new_clip_id(&self) -> ClipId {
        self.new_id
    }
}
impl Command<Project> for SplitClip {
    fn apply(&mut self, p: &mut Project) -> R {
        let (first, second) = p.split_parts(self.id, self.at, self.new_id)?;
        let original = p.replace_clip(first)?;
        if let Err(e) = p.insert_clip_after(self.id, second) {
            p.replace_clip(original)?;
            return Err(e.into());
        }
        self.original = Some(original);
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let original = self.original.take().ok_or_else(missing_state)?;
        p.remove_clip(self.new_id)?;
        p.replace_clip(original)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Dividir clipe"
    }
}

/// Crossfade entre dois clipes sobrepostos da mesma faixa (RF-3.2).
pub struct Crossfade {
    pub a: ClipId,
    pub b: ClipId,
    pub shape: FadeShape,
    previous: Option<(Clip, Clip)>,
}
impl Crossfade {
    pub fn new(a: ClipId, b: ClipId, shape: FadeShape) -> Self {
        Self {
            a,
            b,
            shape,
            previous: None,
        }
    }
}
impl Command<Project> for Crossfade {
    fn apply(&mut self, p: &mut Project) -> R {
        let (x, y) = p.crossfade_parts(self.a, self.b, self.shape)?;
        let px = p.replace_clip(x)?;
        match p.replace_clip(y) {
            Ok(py) => {
                self.previous = Some((px, py));
                Ok(())
            }
            Err(e) => {
                p.replace_clip(px)?;
                Err(e.into())
            }
        }
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let (px, py) = self.previous.take().ok_or_else(missing_state)?;
        p.replace_clip(py)?;
        p.replace_clip(px)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Crossfade"
    }
}

pub struct SetTempo {
    pub tick: Tick,
    pub bpm: f64,
    previous: Option<Option<TempoEvent>>,
}
impl SetTempo {
    pub fn new(tick: Tick, bpm: f64) -> Self {
        Self {
            tick,
            bpm,
            previous: None,
        }
    }
}
impl Command<Project> for SetTempo {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(
            p.tempo_mut()
                .set_tempo(self.tick, self.bpm)
                .map_err(ProjectError::from)?,
        );
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        match self.previous.take().ok_or_else(missing_state)? {
            Some(prev) => p.tempo_mut().set_tempo(prev.tick, prev.bpm).map(|_| ()),
            None => p.tempo_mut().remove_tempo(self.tick).map(|_| ()),
        }
        .map_err(ProjectError::from)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Alterar andamento"
    }
}

pub struct RemoveTempo {
    pub tick: Tick,
    removed: Option<TempoEvent>,
}
impl RemoveTempo {
    pub fn new(tick: Tick) -> Self {
        Self {
            tick,
            removed: None,
        }
    }
}
impl Command<Project> for RemoveTempo {
    fn apply(&mut self, p: &mut Project) -> R {
        self.removed = Some(
            p.tempo_mut()
                .remove_tempo(self.tick)
                .map_err(ProjectError::from)?,
        );
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let e = self.removed.take().ok_or_else(missing_state)?;
        p.tempo_mut()
            .set_tempo(e.tick, e.bpm)
            .map_err(ProjectError::from)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover mudança de andamento"
    }
}

pub struct SetMeter {
    pub tick: Tick,
    pub numerator: u8,
    pub denominator: u8,
    previous: Option<Option<MeterEvent>>,
}
impl SetMeter {
    pub fn new(tick: Tick, numerator: u8, denominator: u8) -> Self {
        Self {
            tick,
            numerator,
            denominator,
            previous: None,
        }
    }
}
impl Command<Project> for SetMeter {
    fn apply(&mut self, p: &mut Project) -> R {
        self.previous = Some(
            p.tempo_mut()
                .set_meter(self.tick, self.numerator, self.denominator)
                .map_err(ProjectError::from)?,
        );
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        match self.previous.take().ok_or_else(missing_state)? {
            Some(prev) => p
                .tempo_mut()
                .set_meter(prev.tick, prev.numerator, prev.denominator)
                .map(|_| ()),
            None => p.tempo_mut().remove_meter(self.tick).map(|_| ()),
        }
        .map_err(ProjectError::from)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Alterar fórmula de compasso"
    }
}

pub struct RemoveMeter {
    pub tick: Tick,
    removed: Option<MeterEvent>,
}
impl RemoveMeter {
    pub fn new(tick: Tick) -> Self {
        Self {
            tick,
            removed: None,
        }
    }
}
impl Command<Project> for RemoveMeter {
    fn apply(&mut self, p: &mut Project) -> R {
        self.removed = Some(
            p.tempo_mut()
                .remove_meter(self.tick)
                .map_err(ProjectError::from)?,
        );
        Ok(())
    }
    fn revert(&mut self, p: &mut Project) -> R {
        let e = self.removed.take().ok_or_else(missing_state)?;
        p.tempo_mut()
            .set_meter(e.tick, e.numerator, e.denominator)
            .map_err(ProjectError::from)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover mudança de compasso"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;
    use shared_kernel::command::History;

    fn roundtrip(p: &mut Project, cmd: Box<dyn Command<Project>>) {
        let before = p.clone();
        let mut h = History::new();
        h.execute(p, cmd, None).unwrap();
        let after = p.clone();
        assert_ne!(before, after, "o comando deve alterar o estado");
        h.undo(p).unwrap();
        assert_eq!(*p, before, "desfazer restaura o estado anterior");
        h.redo(p).unwrap();
        assert_eq!(*p, after, "refazer reproduz o mesmo estado");
    }

    #[test]
    fn every_command_roundtrips() {
        let (mut p, t, s) = project_with_track();
        let c = clip(&s, 0, 100);
        let cid = c.id;
        p.insert_clip(t, c, 0).unwrap();
        let t2 = Track::new_audio("Baixo").unwrap();
        let t2id = t2.id;

        roundtrip(&mut p, Box::new(RenameProject::new("Outro")));
        roundtrip(&mut p, Box::new(CreateTrack::new(t2, 0)));
        roundtrip(&mut p, Box::new(RenameTrack::new(t, "Voz 2")));
        roundtrip(&mut p, Box::new(MoveTrack::new(t, 0)));
        roundtrip(&mut p, Box::new(MoveClip::new(cid, t2id, Tick(480))));
        roundtrip(&mut p, Box::new(SetClipGain::new(cid, Db(-3.0))));
        roundtrip(&mut p, Box::new(TrimClip::new(cid, Tick(482), 40)));
        roundtrip(
            &mut p,
            Box::new(SetClipFades::new(
                cid,
                10,
                20,
                FadeShape::EqualPower,
                FadeShape::Linear,
            )),
        );
        roundtrip(&mut p, Box::new(SetClipPolarity::new(cid, true)));
        roundtrip(&mut p, Box::new(SetClipReversed::new(cid, true)));
        roundtrip(&mut p, Box::new(SplitClip::new(cid, Tick(483))));
        roundtrip(&mut p, Box::new(SetTempo::new(Tick(3840), 90.0)));
        roundtrip(&mut p, Box::new(SetTempo::new(Tick::ZERO, 100.0)));
        roundtrip(&mut p, Box::new(SetMeter::new(Tick(3840), 3, 4)));
        p.tempo_mut().set_tempo(Tick(960), 80.0).unwrap();
        roundtrip(&mut p, Box::new(RemoveTempo::new(Tick(960))));
        p.tempo_mut().set_meter(Tick(3840 * 4), 6, 8).unwrap();
        roundtrip(&mut p, Box::new(RemoveMeter::new(Tick(3840 * 4))));
        roundtrip(&mut p, Box::new(AddSource::new(source(10))));
        roundtrip(&mut p, Box::new(AddClip::new(t, clip(&s, 960, 10))));
        roundtrip(&mut p, Box::new(DeleteClip::new(cid)));
        roundtrip(&mut p, Box::new(DeleteTrack::new(t)));
    }

    #[test]
    fn move_clip_undo_restores_original_index() {
        let (mut p, t, s) = project_with_track();
        let a = clip(&s, 0, 10);
        let b = clip(&s, 100, 10);
        let (aid, bid) = (a.id, b.id);
        p.insert_clip(t, a, 0).unwrap();
        p.insert_clip(t, b, 1).unwrap();
        let mut h = History::new();
        h.execute(&mut p, Box::new(MoveClip::new(aid, t, Tick(2000))), None)
            .unwrap();
        h.undo(&mut p).unwrap();
        let ids: Vec<_> = p.track(t).unwrap().clips.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![aid, bid]);
    }

    #[test]
    fn split_then_undo_restores_single_clip_and_redo_is_stable() {
        let (mut p, t, s) = project_with_track();
        let a = clip(&s, 0, 1000);
        let b = clip(&s, 960, 10);
        let (aid, bid) = (a.id, b.id);
        p.insert_clip(t, a, 0).unwrap();
        p.insert_clip(t, b, 1).unwrap();
        let cmd = SplitClip::new(aid, Tick(10));
        let nid = cmd.new_clip_id();
        let before = p.clone();
        let mut h = History::new();
        h.execute(&mut p, Box::new(cmd), None).unwrap();
        let ids: Vec<_> = p.track(t).unwrap().clips.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![aid, nid, bid],
            "segunda parte logo após a primeira"
        );
        let after = p.clone();
        h.undo(&mut p).unwrap();
        assert_eq!(p, before);
        h.redo(&mut p).unwrap();
        assert_eq!(p, after, "refazer usa o mesmo id");
    }

    #[test]
    fn crossfade_roundtrips_and_rejects_invalid() {
        let (mut p, t, s) = project_with_track();
        let a = clip(&s, 0, 1000);
        let b = clip(&s, 32, 1000);
        let (aid, bid) = (a.id, b.id);
        p.insert_clip(t, a, 0).unwrap();
        p.insert_clip(t, b, 1).unwrap();
        roundtrip(
            &mut p,
            Box::new(Crossfade::new(aid, bid, FadeShape::EqualPower)),
        );
        let before = p.clone();
        let err = Crossfade::new(aid, aid, FadeShape::Linear)
            .apply(&mut p)
            .unwrap_err();
        assert_eq!(err.code, "project.clips_do_not_overlap");
        assert_eq!(p, before);
    }

    #[test]
    fn edit_commands_enforce_invariants_atomically() {
        let (mut p, t, s) = project_with_track();
        let c = clip(&s, 0, 100);
        let cid = c.id;
        p.insert_clip(t, c, 0).unwrap();
        let before = p.clone();
        let err = SetClipFades::new(cid, 60, 41, FadeShape::Linear, FadeShape::Linear)
            .apply(&mut p)
            .unwrap_err();
        assert_eq!(err.code, "project.invalid_fade");
        let err = TrimClip::new(cid, Tick(0), 48_001)
            .apply(&mut p)
            .unwrap_err();
        assert_eq!(err.code, "project.clip_out_of_bounds");
        let err = SplitClip::new(cid, Tick(0)).apply(&mut p).unwrap_err();
        assert_eq!(err.code, "project.invalid_split");
        let err = SetClipPolarity::new(ClipId::new(), true)
            .apply(&mut p)
            .unwrap_err();
        assert_eq!(err.code, "project.clip_not_found");
        assert_eq!(p, before, "falhas não alteram o estado");
    }

    #[test]
    fn invalid_command_reports_stable_code() {
        let (mut p, _, _) = project_with_track();
        let err = RenameTrack::new(TrackId::new(), "x")
            .apply(&mut p)
            .unwrap_err();
        assert_eq!(err.code, "project.track_not_found");
        let err = SetTempo::new(Tick(0), 5000.0).apply(&mut p).unwrap_err();
        assert_eq!(err.code, "project.tempo");
    }

    #[test]
    fn revert_without_apply_is_an_error_not_a_panic() {
        let (mut p, t, _) = project_with_track();
        assert_eq!(
            DeleteTrack::new(t).revert(&mut p).unwrap_err().code,
            "internal.revert_without_apply"
        );
    }
}
