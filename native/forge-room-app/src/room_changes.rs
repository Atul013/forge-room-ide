//! Explicit recipient-scoped saved patch review. Receiving a patch never writes a file.
use eframe::egui;
use forge_room_core::transport::{Action, PatchReview, Snapshot};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};
struct Draft {
    path: String,
    before: String,
    after: String,
    recipient: String,
}
#[derive(Default)]
pub struct Changes {
    recipient: String,
    draft: Option<Draft>,
    loading: Option<Receiver<Result<Draft, String>>>,
    review: crate::ai_workflow::Workflow,
    review_id: Option<u64>,
    notice: Option<String>,
    pub changed: bool,
}
impl Changes {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        root: Option<&Path>,
        trusted: bool,
        dirty: &[PathBuf],
        online: bool,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Some(receiver) = &self.loading
            && let Ok(result) = receiver.try_recv()
        {
            match result {
                Ok(draft) => self.draft = Some(draft),
                Err(error) => self.notice = Some(error),
            }
            self.loading = None;
        }
        if self
            .review_id
            .is_some_and(|id| !snapshot.patches.iter().any(|p| p.id == id))
        {
            self.review.preview = None;
            self.review_id = None;
        }
        egui::CollapsingHeader::new("Share / review saved patches").show(ui,|ui|{
            ui.label("Nothing is shared until you choose a saved Git-tracked file, review both versions and send it to one recipient. Includes its relative path and full before/after text. No unsaved buffers or terminal output.");
            if let Some(notice)=&self.notice{ui.label(notice);}
            egui::ComboBox::from_id_salt("network-patch-recipient").selected_text(label(snapshot,&self.recipient)).show_ui(ui,|ui|{for member in snapshot.members.iter().filter(|m|m.id!=snapshot.you){ui.selectable_value(&mut self.recipient,member.id.clone(),&member.name);}});
            if ui.add_enabled(trusted&&online&&root.is_some()&&!self.recipient.is_empty()&&self.loading.is_none(),egui::Button::new("Prepare saved file vs Git HEAD...")).clicked()
                && let Some(root)=root && let Some(path)=rfd::FileDialog::new().set_directory(root).pick_file(){
                let root=root.to_path_buf();let recipient=self.recipient.clone();let (sender,receiver)=mpsc::sync_channel(1);self.loading=Some(receiver);let context=ui.ctx().clone();
                std::thread::spawn(move||{let result=crate::ai_workflow::saved_patch(&root,&path).map(|(path,before,after)|Draft {path,before,after,recipient});let _=sender.send(result);context.request_repaint();});
            }
            if self.loading.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
            let mut discard=false;
            if let Some(draft)=&self.draft {
                ui.label(format!("{} -> {}",draft.path,label(snapshot,&draft.recipient)));
                versions(ui,&draft.before,&draft.after,"outgoing-patch");
                if ui.add_enabled(online,egui::Button::new("Send this reviewed patch to recipient")).clicked(){actions.push(Action::SharePatch {recipient:draft.recipient.clone(),path:draft.path.clone(),before:draft.before.clone(),after:draft.after.clone()});discard=true;}
                discard|=ui.button("Discard outgoing patch").clicked();
            }
            if discard{self.draft=None;}
            for patch in &snapshot.patches{
                ui.label(format!("Patch #{} | {} | {} -> {} | {}",patch.id,patch.path,label(snapshot,&patch.actor),label(snapshot,&patch.recipient),match patch.review {PatchReview::Pending=>"awaiting recipient review",PatchReview::Reviewed=>"reviewed",PatchReview::Declined=>"declined"}));
                if patch.actor==snapshot.you&&ui.button(format!("Revoke patch #{}",patch.id)).clicked(){actions.push(Action::RevokePatch {id:patch.id});}
                if patch.recipient==snapshot.you&&ui.add_enabled(trusted&&online&&root.is_some(),egui::Button::new(format!("Review patch #{} for this workspace",patch.id))).clicked()&&let Some(root)=root{
                    match crate::ai_workflow::Workflow::received_patch(root,patch){Ok(review)=>{self.review=review;self.review_id=Some(patch.id);},Err(error)=>self.notice=Some(error)}
                }
            }
            let mut apply=false;
            if let Some(preview)=&self.review.preview{for file in &preview.files{ui.label(file.path.display().to_string());versions(ui,&file.before,&file.after,"received-patch");}
                if let Some(id)=self.review_id {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(online,egui::Button::new("Mark patch reviewed")).clicked(){actions.push(Action::ReviewPatch{id,decision:PatchReview::Reviewed});self.notice=Some("Review receipt sent to the patch sender".into());}
                        if ui.add_enabled(online,egui::Button::new("Decline patch")).clicked(){actions.push(Action::ReviewPatch{id,decision:PatchReview::Declined});self.notice=Some("Decline receipt sent to the patch sender".into());}
                    });
                }
                apply=ui.add_enabled(trusted&&online&&self.review_id.is_some(),egui::Button::new("Apply this received patch locally")).clicked();}
            if apply&&let Some(root)=root{match self.review.apply(root,dirty,false){Ok(())=>{self.changed=true;self.notice=Some("Received patch applied locally; guarded undo is available".into());},Err(error)=>self.notice=Some(error)}}
            if self.review.undo.is_some()&&ui.add_enabled(trusted,egui::Button::new("Undo last received patch")).clicked()&&let Some(root)=root{match self.review.apply(root,dirty,true){Ok(())=>{self.changed=true;self.notice=Some("Received patch was undone".into());},Err(error)=>self.notice=Some(error)}}
            ui.small("Credential-path and content filters are best effort: inspect both versions. Only sender and recipient receive this patch. Sender revocation or either member leaving ends further delivery; copies already received cannot be withdrawn. Apply requires a matching local base and refuses dirty or changed files.");
        });
        actions
    }
}
fn label(snapshot: &Snapshot, id: &str) -> String {
    snapshot
        .members
        .iter()
        .find(|m| m.id == id)
        .map(|m| format!("{} ({})", m.name, id.chars().take(8).collect::<String>()))
        .unwrap_or_else(|| "Choose joined recipient".into())
}
fn versions(ui: &mut egui::Ui, before: &str, after: &str, id: &str) {
    for (caption, text) in [("Before", before), ("After", after)] {
        ui.label(caption);
        let mut text = text;
        egui::ScrollArea::vertical()
            .id_salt((id, caption))
            .max_height(120.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .interactive(false)
                        .desired_width(f32::INFINITY),
                );
            });
    }
}
