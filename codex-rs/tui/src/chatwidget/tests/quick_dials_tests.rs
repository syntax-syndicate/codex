//! Save, Clear, and confirmation layouts keep destructive preset actions explicit.

use super::*;
use crate::chatwidget::quick_dials::QuickDialEditorTarget;
use crate::chatwidget::quick_dials::QuickDialPickerMode;
use codex_config::types::QuickDial;

#[tokio::test]
async fn saved_quick_dials_save_and_clear_picker_snapshots() {
    const DRAFT: &str = "Review the current changes\nfor safety  ";
    let (mut chat, mut events, _ops) =
        make_chatwidget_manual(/*model_override*/ Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    chat.local_settings.tui.quick_dials = [(
        "1".into(),
        QuickDial {
            input: "/model gpt-5.5 --reasoning=high --speed=fast".into(),
            label: Some("coding".into()),
        },
    )]
    .into();
    chat.bottom_pane.apply_external_edit(DRAFT.into());
    chat.dispatch_command(SlashCommand::Presets);
    assert_chatwidget_snapshot!(
        "saved_quick_dials_recall",
        render_bottom_popup(&chat, /*width*/ 80)
    );
    chat.handle_key_event(KeyCode::Esc.into());
    while events.try_recv().is_ok() {}
    chat.open_quick_dial_editor(QuickDialEditorTarget::PickSlot, /*dial*/ None);
    let editor = render_bottom_popup(&chat, /*width*/ 80);
    assert_chatwidget_snapshot!("saved_quick_dials_editor", editor.clone());
    chat.handle_key_event(KeyCode::Enter.into());
    let edited = std::iter::from_fn(|| events.try_recv().ok())
        .find_map(|event| match event {
            AppEvent::OpenQuickDialSavePicker(dial) => Some(dial),
            _ => None,
        })
        .expect("editor submits a preset draft");
    assert_eq!(
        edited,
        QuickDial {
            input: DRAFT.into(),
            label: None,
        }
    );
    chat.open_quick_dial_save_picker(edited);
    assert_chatwidget_snapshot!(
        "saved_quick_dials_save",
        render_bottom_popup(&chat, /*width*/ 80)
    );
    chat.handle_key_event(KeyCode::Esc.into());
    assert_eq!(render_bottom_popup(&chat, /*width*/ 80), editor);
    chat.handle_key_event(KeyCode::Esc.into());

    let saved = chat.local_settings.tui.quick_dials["1"].clone();
    chat.open_quick_dial_editor(QuickDialEditorTarget::Slot(/*slot*/ 1), Some(saved.clone()));
    chat.handle_key_event(KeyCode::Enter.into());
    assert_matches!(
        std::iter::from_fn(|| events.try_recv().ok())
            .find(|event| matches!(event, AppEvent::ConfirmQuickDial { .. })),
        Some(AppEvent::ConfirmQuickDial {
            slot: 1,
            dial: Some(actual),
        }) if actual == saved
    );
    chat.handle_key_event(KeyCode::Esc.into());

    chat.open_quick_dials_picker(QuickDialPickerMode::Clear);
    assert_chatwidget_snapshot!(
        "saved_quick_dials_clear",
        render_bottom_popup(&chat, /*width*/ 80)
    );
    chat.handle_key_event(KeyCode::Esc.into());
    while events.try_recv().is_ok() {}
    let replacement = QuickDial {
        input: "/model gpt-5.6-terra --reasoning=high --speed=fast".into(),
        label: None,
    };
    chat.confirm_quick_dial(/*slot*/ 2, Some(replacement.clone()));
    assert_chatwidget_snapshot!(
        "saved_quick_dials_confirm_save",
        render_bottom_popup(&chat, /*width*/ 52)
    );
    chat.handle_key_event(KeyCode::Esc.into());
    assert!(
        !std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, AppEvent::SaveQuickDial { .. }))
    );

    chat.confirm_quick_dial(/*slot*/ 1, Some(replacement.clone()));
    assert_chatwidget_snapshot!(
        "saved_quick_dials_confirm_replace",
        render_bottom_popup(&chat, /*width*/ 52)
    );
    chat.handle_key_event(KeyCode::Enter.into());
    assert_matches!(
        events.try_recv(),
        Ok(AppEvent::SaveQuickDial {
            slot: 1,
            dial: Some(actual),
        }) if actual == replacement
    );

    chat.confirm_quick_dial(/*slot*/ 1, /*dial*/ None);
    assert_chatwidget_snapshot!(
        "saved_quick_dials_confirm_delete",
        render_bottom_popup(&chat, /*width*/ 80)
    );
    chat.handle_key_event(KeyCode::Esc.into());
    assert!(
        !std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, AppEvent::SaveQuickDial { .. }))
    );

    chat.recall_quick_dial(/*slot*/ 1);
    assert_eq!(chat.bottom_pane.composer_text(), DRAFT);
    assert_eq!(
        chat.queued_user_message_texts(),
        ["/model gpt-5.5 --reasoning=high --speed=fast"]
    );

    chat.set_reasoning_effort(Some(ReasoningEffortConfig::High));
    chat.effective_service_tier = Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.into());
    assert_eq!(
        chat.current_model_quick_dial(),
        Ok(QuickDial {
            input: "/model gpt-5.5 --reasoning=high --speed=standard".into(),
            label: None,
        })
    );
    let expected_model = chat.current_model_quick_dial().expect("model command");
    chat.open_quick_dial_prefills(QuickDialEditorTarget::PickSlot);
    chat.handle_key_event(KeyCode::Enter.into());
    assert_matches!(
        std::iter::from_fn(|| events.try_recv().ok())
            .find(|event| matches!(event, AppEvent::OpenQuickDialEditor { .. })),
        Some(AppEvent::OpenQuickDialEditor {
            target: QuickDialEditorTarget::PickSlot,
            dial: Some(actual),
        }) if actual == expected_model
    );
    chat.handle_key_event(KeyCode::Esc.into());
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::Custom("removed-effort".into())));
    assert_eq!(
        chat.current_model_quick_dial(),
        Err("Choose an available reasoning effort with /model before saving.")
    );
    chat.set_reasoning_effort(/*effort*/ None);
    chat.effective_service_tier = Some("removed-speed".into());
    assert_eq!(
        chat.current_model_quick_dial(),
        Err("Choose an available speed with /model before saving.")
    );
}
