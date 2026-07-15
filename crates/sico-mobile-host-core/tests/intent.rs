use std::io::{Cursor, Read};

use sico_mobile_host_core::{
    AndroidIntent, AndroidIntentAction, IntentDecision, IntentError, copy_package_stream,
    validate_intent,
};

#[test]
fn view_send_and_picker_accept_one_content_sapp() {
    for action in [
        AndroidIntentAction::View,
        AndroidIntentAction::Send,
        AndroidIntentAction::PickerResult,
    ] {
        assert!(matches!(
            validate_intent(&package_intent(action)),
            Ok(IntentDecision::CopyContentUri(_))
        ));
    }
}

#[test]
fn scheme_mime_name_and_clip_spoofing_fail_closed() {
    let mut intent = package_intent(AndroidIntentAction::View);
    intent.uri = Some("file:///sdcard/app.sapp".to_owned());
    assert_eq!(validate_intent(&intent), Err(IntentError::InvalidUri));
    intent = package_intent(AndroidIntentAction::View);
    intent.mime_type = Some("application/zip".to_owned());
    assert_eq!(validate_intent(&intent), Err(IntentError::InvalidMime));
    intent = package_intent(AndroidIntentAction::View);
    intent.display_name = Some("app.zip".to_owned());
    assert_eq!(validate_intent(&intent), Err(IntentError::InvalidName));
    intent = package_intent(AndroidIntentAction::View);
    intent.clip_items = 2;
    assert_eq!(validate_intent(&intent), Err(IntentError::MultipleItems));
}

#[test]
fn deep_link_only_opens_system_picker() {
    let accepted = AndroidIntent {
        action: AndroidIntentAction::DeepLink,
        uri: Some("sico://open".to_owned()),
        mime_type: None,
        display_name: None,
        clip_items: 0,
    };
    assert_eq!(
        validate_intent(&accepted),
        Ok(IntentDecision::OpenSystemPicker)
    );
    let mut rejected = accepted;
    rejected.uri = Some("sico://open?uri=https://evil/app.sapp".to_owned());
    assert_eq!(validate_intent(&rejected), Err(IntentError::InvalidUri));
}

#[test]
fn stream_is_read_once_and_limit_plus_one_is_rejected() {
    let mut reader = CountingReader {
        inner: Cursor::new(b"package".to_vec()),
        reads: 0,
    };
    assert_eq!(copy_package_stream(&mut reader).unwrap(), b"package");
    assert!(reader.reads > 0);
    let mut too_large = std::io::repeat(0).take(64 * 1024 * 1024 + 1);
    assert_eq!(
        copy_package_stream(&mut too_large),
        Err(IntentError::StreamTooLarge)
    );
}

fn package_intent(action: AndroidIntentAction) -> AndroidIntent {
    AndroidIntent {
        action,
        uri: Some("content://documents/tree/answer.sapp".to_owned()),
        mime_type: Some("application/vnd.sico.sapp".to_owned()),
        display_name: Some("answer.sapp".to_owned()),
        clip_items: 1,
    }
}

struct CountingReader {
    inner: Cursor<Vec<u8>>,
    reads: usize,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        self.inner.read(buffer)
    }
}
