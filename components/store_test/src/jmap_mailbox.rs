use std::{collections::HashMap, iter::FromIterator};

use jmap::{
    changes::{JMAPChangesRequest, JMAPState},
    id::JMAPIdSerialize,
    json::JSONValue,
    JMAPComparator, JMAPFilter, JMAPGet, JMAPQueryRequest, JMAPSet,
};
use jmap_mail::{
    get::{JMAPMailGet, JMAPMailGetArguments},
    import::JMAPMailImport,
    mailbox::{
        JMAPMailMailbox, JMAPMailboxComparator, JMAPMailboxFilterCondition, JMAPMailboxProperties,
        JMAPMailboxQueryArguments, JMAPMailboxSetArguments,
    },
    set::JMAPMailSet,
};

use store::{AccountId, JMAPId, JMAPIdPrefix, JMAPStore, Store};

const TEST_MAILBOXES: &[u8] = br#"
[
    {
        "id": "inbox",
        "name": "Inbox",
        "role": "INBOX",
        "sortOrder": 5,
        "children": [
            {
                "name": "Level 1",
                "id": "1",
                "sortOrder": 4,
                "children": [
                    {
                        "name": "Sub-Level 1.1",
                        "id": "1.1",

                        "sortOrder": 3,
                        "children": [
                            {
                                "name": "Z-Sub-Level 1.1.1",
                                "id": "1.1.1",
                                "sortOrder": 2,
                                "children": [
                                    {
                                        "name": "X-Sub-Level 1.1.1.1",
                                        "id": "1.1.1.1",
                                        "sortOrder": 1,
                                        "children": [
                                            {
                                                "name": "Y-Sub-Level 1.1.1.1.1",
                                                "id": "1.1.1.1.1",
                                                "sortOrder": 0
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        "name": "Sub-Level 1.2",
                        "id": "1.2",
                        "sortOrder": 7,
                        "children": [
                            {
                                "name": "Z-Sub-Level 1.2.1",
                                "id": "1.2.1",
                                "sortOrder": 6
                            }
                        ]
                    }
                ]
            },
            {
                "name": "Level 2",
                "id": "2",
                "sortOrder": 8
            },
            {
                "name": "Level 3",
                "id": "3",
                "sortOrder": 9
            }
        ]
    },
    {
        "id": "sent",
        "name": "Sent",
        "role": "SENT",
        "sortOrder": 15
    },
    {
        "id": "drafts",
        "name": "Drafts",
        "role": "DRAFTS",
        "sortOrder": 14
    },
    {
        "id": "trash",
        "name": "Trash",
        "role": "TRASH",
        "sortOrder": 13
    },
    {
        "id": "spam",
        "name": "Spam",
        "role": "SPAM",
        "sortOrder": 12,
        "children": [{
            "id": "spam1",
            "name": "Work Spam",
            "sortOrder": 11,
            "children": [{
                "id": "spam2",
                "name": "Friendly Spam",
                "sortOrder": 10
            }]
        }]
    }
]
"#;

pub fn jmap_mailbox<T>(mail_store: &JMAPStore<T>, account_id: AccountId)
where
    T: for<'x> Store<'x> + 'static,
{
    let mut id_map = HashMap::new();
    create_nested_mailboxes(
        mail_store,
        None,
        serde_json::from_slice(TEST_MAILBOXES).unwrap(),
        &mut id_map,
        account_id,
    );

    // Sort by name
    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::None,
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        [
            "drafts",
            "spam2",
            "inbox",
            "1",
            "2",
            "3",
            "sent",
            "spam",
            "1.1",
            "1.2",
            "trash",
            "spam1",
            "1.1.1.1",
            "1.1.1.1.1",
            "1.1.1",
            "1.2.1"
        ]
    );

    // Sort by name as tree
    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::None,
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: true,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        [
            "drafts",
            "inbox",
            "1",
            "1.1",
            "1.1.1",
            "1.1.1.1",
            "1.1.1.1.1",
            "1.2",
            "1.2.1",
            "2",
            "3",
            "sent",
            "spam",
            "spam1",
            "spam2",
            "trash"
        ]
    );

    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Name(
                    "level".to_string()
                )),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: true,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        [
            "1",
            "1.1",
            "1.1.1",
            "1.1.1.1",
            "1.1.1.1.1",
            "1.2",
            "1.2.1",
            "2",
            "3"
        ]
    );

    // Filter as tree
    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Name("spam".to_string())),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: true,
                    filter_as_tree: true,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["spam", "spam1", "spam2"]
    );

    assert!(mail_store
        .mailbox_query(JMAPQueryRequest {
            account_id,
            filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Name("level".to_string())),
            sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
            position: 0,
            anchor: None,
            anchor_offset: 0,
            limit: 100,
            calculate_total: true,
            arguments: JMAPMailboxQueryArguments {
                sort_as_tree: true,
                filter_as_tree: true,
            },
        })
        .unwrap()
        .eval_unwrap_array("/ids")
        .is_empty());

    // Role filters
    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Role(
                    "inbox".to_string()
                )),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["inbox"]
    );

    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::HasAnyRole),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["drafts", "inbox", "sent", "spam", "trash"]
    );

    // Duplicate role
    assert!(mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: JSONValue::Null,
            update: HashMap::from_iter([(
                get_mailbox_id(&id_map, "sent"),
                HashMap::from_iter([("role".to_string(), "INBOX".to_string().into())]).into(),
            )])
            .into(),
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap()
        .eval_unwrap_object("/notUpdated")
        .remove(&get_mailbox_id(&id_map, "sent"))
        .is_some());

    // Duplicate name
    assert!(mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: JSONValue::Null,
            update: HashMap::from_iter([(
                get_mailbox_id(&id_map, "2"),
                HashMap::from_iter([("name".to_string(), "Level 3".to_string().into())]).into(),
            )])
            .into(),
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap()
        .eval_unwrap_object("/notUpdated")
        .remove(&get_mailbox_id(&id_map, "2"))
        .is_some());

    // Circular relationship
    assert!(mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: JSONValue::Null,
            update: HashMap::from_iter([(
                get_mailbox_id(&id_map, "1"),
                HashMap::from_iter([(
                    "parentId".to_string(),
                    get_mailbox_id(&id_map, "1.1.1.1.1").into()
                )])
                .into(),
            )])
            .into(),
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap()
        .eval_unwrap_object("/notUpdated")
        .remove(&get_mailbox_id(&id_map, "1"))
        .is_some());

    assert!(mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: JSONValue::Null,
            update: HashMap::from_iter([(
                get_mailbox_id(&id_map, "1"),
                HashMap::from_iter([("parentId".to_string(), get_mailbox_id(&id_map, "1").into())])
                    .into(),
            )])
            .into(),
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap()
        .eval_unwrap_object("/notUpdated")
        .remove(&get_mailbox_id(&id_map, "1"))
        .is_some());

    // Invalid parent ID
    assert!(mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: JSONValue::Null,
            update: HashMap::from_iter([(
                get_mailbox_id(&id_map, "1"),
                HashMap::from_iter([("parentId".to_string(), JMAPId::MAX.to_jmap_string().into())])
                    .into(),
            )])
            .into(),
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap()
        .eval_unwrap_object("/notUpdated")
        .remove(&get_mailbox_id(&id_map, "1"))
        .is_some());

    // Get state
    let state = mail_store
        .mailbox_changes(JMAPChangesRequest {
            account_id,
            since_state: JMAPState::Initial,
            max_changes: 0,
        })
        .unwrap()
        .eval_unwrap_jmap_state("/newState");

    // Rename and move mailbox
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: HashMap::from_iter([(
                    get_mailbox_id(&id_map, "1.1.1.1.1"),
                    HashMap::from_iter([
                        ("name".to_string(), "Renamed and moved".to_string().into()),
                        ("parentId".to_string(), get_mailbox_id(&id_map, "2").into())
                    ])
                    .into(),
                )])
                .into(),
                destroy: JSONValue::Null,
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap()
            .eval("/notUpdated")
            .unwrap(),
        JSONValue::Null
    );

    // Verify changes
    let state = mail_store
        .mailbox_changes(JMAPChangesRequest {
            account_id,
            since_state: state,
            max_changes: 0,
        })
        .unwrap();
    assert_eq!(state.eval_unwrap_unsigned_int("/totalChanges"), 1);
    assert!(state.eval_unwrap_array("/updated").len() == 1);
    assert!(
        state.eval("/updatedProperties").is_err(),
        "{:?}",
        state.eval("/updatedProperties").unwrap()
    );
    let state = state.eval_unwrap_jmap_state("/newState");

    // Insert email into Inbox
    let message_id = mail_store
        .mail_import_blob(
            account_id,
            b"From: test@test.com\nSubject: hey\n\ntest".to_vec(),
            vec![JMAPId::from_jmap_string(&get_mailbox_id(&id_map, "inbox"))
                .unwrap()
                .get_document_id()],
            vec![],
            None,
        )
        .unwrap()
        .eval_unwrap_string("/id");

    // Only email properties must have changed
    let state = mail_store
        .mailbox_changes(JMAPChangesRequest {
            account_id,
            since_state: state,
            max_changes: 0,
        })
        .unwrap();
    assert_eq!(state.eval_unwrap_unsigned_int("/totalChanges"), 1);
    assert_eq!(
        state.eval("/updated").unwrap(),
        vec![get_mailbox_id(&id_map, "inbox").into()].into()
    );
    assert_eq!(
        state.eval("/updatedProperties").unwrap(),
        JSONValue::Array(vec![
            JMAPMailboxProperties::TotalEmails.into(),
            JMAPMailboxProperties::UnreadEmails.into(),
            JMAPMailboxProperties::TotalThreads.into(),
            JMAPMailboxProperties::UnreadThreads.into(),
        ])
    );
    let state = state.eval_unwrap_jmap_state("/newState");

    // Move email from Inbox to Trash
    assert_eq!(
        mail_store
            .mail_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: HashMap::from_iter([(
                    message_id.clone(),
                    HashMap::from_iter([(
                        "mailboxIds".to_string(),
                        HashMap::from_iter([(get_mailbox_id(&id_map, "trash"), true.into())])
                            .into()
                    )])
                    .into(),
                )])
                .into(),
                destroy: JSONValue::Null,
                arguments: (),
            })
            .unwrap()
            .eval("/notUpdated")
            .unwrap(),
        JSONValue::Null
    );

    // E-mail properties of both Inbox and Trash must have changed
    let state = mail_store
        .mailbox_changes(JMAPChangesRequest {
            account_id,
            since_state: state,
            max_changes: 0,
        })
        .unwrap();
    assert_eq!(state.eval_unwrap_unsigned_int("/totalChanges"), 2);
    let mut folder_ids = vec![
        JMAPId::from_jmap_string(&get_mailbox_id(&id_map, "trash")).unwrap(),
        JMAPId::from_jmap_string(&get_mailbox_id(&id_map, "inbox")).unwrap(),
    ];
    let mut updated_ids = state
        .eval_unwrap_array("/updated")
        .into_iter()
        .map(|i| i.to_jmap_id().unwrap())
        .collect::<Vec<_>>();
    updated_ids.sort_unstable();
    folder_ids.sort_unstable();
    assert_eq!(updated_ids, folder_ids);
    assert_eq!(
        state.eval("/updatedProperties").unwrap(),
        JSONValue::Array(vec![
            JMAPMailboxProperties::TotalEmails.into(),
            JMAPMailboxProperties::UnreadEmails.into(),
            JMAPMailboxProperties::TotalThreads.into(),
            JMAPMailboxProperties::UnreadThreads.into(),
        ])
    );

    // Deleting folders with children is not allowed
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: JSONValue::Null,
                destroy: vec![get_mailbox_id(&id_map, "1").into()].into(),
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap()
            .eval("/destroyed")
            .unwrap(),
        JSONValue::Null,
    );

    // Deleting folders with contents is not allowed (unless remove_emails is true)
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: JSONValue::Null,
                destroy: vec![get_mailbox_id(&id_map, "trash").into()].into(),
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap()
            .eval("/destroyed")
            .unwrap(),
        JSONValue::Null,
    );

    // Delete Trash folder and its contents
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: JSONValue::Null,
                destroy: vec![get_mailbox_id(&id_map, "trash").into()].into(),
                arguments: JMAPMailboxSetArguments {
                    remove_emails: true,
                },
            })
            .unwrap()
            .eval("/destroyed/0")
            .unwrap(),
        get_mailbox_id(&id_map, "trash").into(),
    );

    // Verify that Trash folder and its contents are gone
    assert_eq!(
        mail_store
            .mailbox_get(JMAPGet {
                account_id,
                ids: vec![JMAPId::from_jmap_string(&get_mailbox_id(&id_map, "trash")).unwrap()]
                    .into(),
                properties: None,
                arguments: (),
            })
            .unwrap()
            .eval("/notFound")
            .unwrap(),
        vec![get_mailbox_id(&id_map, "trash").into()].into()
    );
    assert_eq!(
        mail_store
            .mail_get(JMAPGet {
                account_id,
                ids: vec![JMAPId::from_jmap_string(&message_id).unwrap()].into(),
                properties: None,
                arguments: JMAPMailGetArguments::default(),
            })
            .unwrap()
            .eval("/notFound")
            .unwrap(),
        vec![message_id.into()].into()
    );

    // Check search results after changing folder properties
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: HashMap::from_iter([(
                    get_mailbox_id(&id_map, "drafts"),
                    HashMap::from_iter([
                        ("name".to_string(), "Borradores".to_string().into()),
                        ("role".to_string(), JSONValue::Null),
                        ("sortOrder".to_string(), 100u64.into()),
                        ("parentId".to_string(), get_mailbox_id(&id_map, "2").into())
                    ])
                    .into(),
                )])
                .into(),
                destroy: JSONValue::Null,
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap()
            .eval("/notUpdated")
            .unwrap(),
        JSONValue::Null
    );

    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::and(vec![
                    JMAPFilter::condition(JMAPMailboxFilterCondition::Name(
                        "Borradores".to_string()
                    )),
                    JMAPFilter::condition(JMAPMailboxFilterCondition::ParentId(
                        JMAPId::from_jmap_string(&get_mailbox_id(&id_map, "2")).unwrap() + 1
                    )),
                    JMAPFilter::not(vec![JMAPFilter::condition(
                        JMAPMailboxFilterCondition::HasAnyRole
                    )]),
                ]),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["drafts",]
    );

    assert!(mail_store
        .mailbox_query(JMAPQueryRequest {
            account_id,
            filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Name("Drafts".to_string())),
            sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
            position: 0,
            anchor: None,
            anchor_offset: 0,
            limit: 100,
            calculate_total: true,
            arguments: JMAPMailboxQueryArguments {
                sort_as_tree: false,
                filter_as_tree: false,
            },
        })
        .unwrap()
        .eval_unwrap_array("/ids")
        .is_empty());

    assert!(mail_store
        .mailbox_query(JMAPQueryRequest {
            account_id,
            filter: JMAPFilter::condition(JMAPMailboxFilterCondition::Role("drafts".to_string())),
            sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
            position: 0,
            anchor: None,
            anchor_offset: 0,
            limit: 100,
            calculate_total: true,
            arguments: JMAPMailboxQueryArguments {
                sort_as_tree: false,
                filter_as_tree: false,
            },
        })
        .unwrap()
        .eval_unwrap_array("/ids")
        .is_empty());

    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::ParentId(0)),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["inbox", "sent", "spam",]
    );

    assert_eq!(
        mail_store
            .mailbox_query(JMAPQueryRequest {
                account_id,
                filter: JMAPFilter::condition(JMAPMailboxFilterCondition::HasAnyRole),
                sort: vec![JMAPComparator::ascending(JMAPMailboxComparator::Name)],
                position: 0,
                anchor: None,
                anchor_offset: 0,
                limit: 100,
                calculate_total: true,
                arguments: JMAPMailboxQueryArguments {
                    sort_as_tree: false,
                    filter_as_tree: false,
                },
            })
            .unwrap()
            .eval_unwrap_array("/ids")
            .into_iter()
            .map(|id| id_map.get(&id.to_jmap_id().unwrap()).unwrap())
            .collect::<Vec<_>>(),
        ["inbox", "sent", "spam",]
    );
}

fn get_mailbox_id(id_map: &HashMap<JMAPId, String>, local_id: &str) -> String {
    id_map
        .keys()
        .find(|id| id_map.get(id).unwrap() == local_id)
        .unwrap()
        .clone()
        .to_jmap_string()
}

fn create_nested_mailboxes<T>(
    mail_store: &JMAPStore<T>,
    parent_id: Option<JMAPId>,
    mailboxes: Vec<JSONValue>,
    id_map: &mut HashMap<JMAPId, String>,
    account_id: AccountId,
) where
    T: for<'x> Store<'x> + 'static,
{
    for (mailbox_num, mut mailbox) in mailboxes.into_iter().enumerate() {
        let mut children = None;
        let mut local_id = None;

        if let JSONValue::Object(mailbox) = &mut mailbox {
            children = mailbox.remove("children");
            local_id = mailbox.remove("id").unwrap().unwrap_string();

            if let Some(parent_id) = parent_id {
                mailbox.insert("parentId".to_string(), parent_id.to_jmap_string().into());
            }
        }
        let mailbox_num = format!("b{}", mailbox_num);
        let result = mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: HashMap::from_iter([(mailbox_num.clone(), mailbox)]).into(),
                update: JSONValue::Null,
                destroy: JSONValue::Null,
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap();

        assert_eq!(result.eval("/notCreated").unwrap(), JSONValue::Null);

        let mailbox_id = result.eval_unwrap_jmap_id(&format!("/created/{}/id", mailbox_num));

        if let Some(children) = children {
            create_nested_mailboxes(
                mail_store,
                mailbox_id.into(),
                children.unwrap_array().unwrap(),
                id_map,
                account_id,
            );
        }

        assert!(id_map.insert(mailbox_id, local_id.unwrap()).is_none());
    }
}

pub fn insert_mailbox<T>(
    mail_store: &JMAPStore<T>,
    account_id: AccountId,
    name: &str,
    role: &str,
) -> JMAPId
where
    T: for<'x> Store<'x> + 'static,
{
    let result = mail_store
        .mailbox_set(JMAPSet {
            account_id,
            if_in_state: None,
            create: HashMap::from_iter([(
                "my_id".to_string(),
                HashMap::from_iter([
                    ("name".to_string(), name.to_string().into()),
                    ("role".to_string(), role.to_string().into()),
                ])
                .into(),
            )])
            .into(),
            update: JSONValue::Null,
            destroy: JSONValue::Null,
            arguments: JMAPMailboxSetArguments {
                remove_emails: false,
            },
        })
        .unwrap();

    assert_eq!(result.eval("/notCreated").unwrap(), JSONValue::Null);

    result.eval_unwrap_jmap_id("/created/my_id/id")
}

pub fn update_mailbox<T>(
    mail_store: &JMAPStore<T>,
    account_id: AccountId,
    jmap_id: JMAPId,
    ref_id: u32,
    seq_id: u32,
) where
    T: for<'x> Store<'x> + 'static,
{
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: HashMap::from_iter([(
                    jmap_id.to_jmap_string(),
                    HashMap::from_iter([
                        (
                            "name".to_string(),
                            format!("Mailbox {}_{}", ref_id, seq_id).into()
                        ),
                        ("role".to_string(), format!("{}_{}", ref_id, seq_id).into()),
                        ("sortOrder".to_string(), ((ref_id * 100) + seq_id).into())
                    ])
                    .into(),
                )])
                .into(),
                destroy: JSONValue::Null,
                arguments: JMAPMailboxSetArguments {
                    remove_emails: false,
                },
            })
            .unwrap()
            .eval("/notUpdated")
            .unwrap(),
        JSONValue::Null
    );
}

pub fn delete_mailbox<T>(mail_store: &JMAPStore<T>, account_id: AccountId, jmap_id: JMAPId)
where
    T: for<'x> Store<'x> + 'static,
{
    assert_eq!(
        mail_store
            .mailbox_set(JMAPSet {
                account_id,
                if_in_state: None,
                create: JSONValue::Null,
                update: JSONValue::Null,
                destroy: vec![jmap_id.to_jmap_string().into()].into(),
                arguments: JMAPMailboxSetArguments {
                    remove_emails: true,
                },
            })
            .unwrap()
            .eval("/notDestroyed")
            .unwrap(),
        JSONValue::Null
    );
}
