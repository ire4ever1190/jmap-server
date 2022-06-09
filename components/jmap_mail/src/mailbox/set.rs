use crate::mail::set::JMAPSetMail;
use crate::mail::MessageField;
use jmap::error::set::{SetError, SetErrorType};
use jmap::jmap_store::set::{SetHelper, SetObject};
use jmap::jmap_store::Object;
use jmap::orm::{serialize::JMAPOrm, TinyORM};
use jmap::request::set::{SetRequest, SetResponse};
use jmap::request::ResultReference;
use jmap::types::jmap::JMAPId;

use store::core::collection::Collection;
use store::core::document::Document;
use store::core::error::StoreError;
use store::core::tag::Tag;
use store::core::JMAPIdPrefix;
use store::parking_lot::MutexGuard;
use store::read::comparator::Comparator;
use store::read::filter::{ComparisonOperator, Filter, Query};
use store::read::FilterMapper;
use store::Store;
use store::{DocumentId, JMAPStore, LongInteger};

use super::schema::{Mailbox, Property, Value};

//TODO mailbox id 0 is inbox and cannot be deleted

#[derive(Debug, Clone, Default)]
pub struct SetArguments {
    pub on_destroy_remove_emails: Option<bool>,
}

impl SetObject for Mailbox {
    type SetArguments = SetArguments;

    type NextCall = ();

    fn eval_id_references(&mut self, mut fnc: impl FnMut(&str) -> Option<JMAPId>) {
        for (_, entry) in self.properties.iter_mut() {
            if let Value::IdReference { value } = entry {
                if let Some(value) = fnc(value) {
                    *entry = Value::Id { value };
                }
            }
        }
    }

    fn eval_result_references(
        &mut self,
        mut fnc: impl FnMut(&ResultReference) -> Option<Vec<u64>>,
    ) {
        for (_, entry) in self.properties.iter_mut() {
            if let Value::ResultReference { value } = entry {
                if let Some(value) = fnc(value).and_then(|mut v| v.pop()) {
                    *entry = Value::Id {
                        value: value.into(),
                    };
                }
            }
        }
    }
}

pub trait JMAPSetMailbox<T>
where
    T: for<'x> Store<'x> + 'static,
{
    fn mailbox_set(&self, request: SetRequest<Mailbox>) -> jmap::Result<SetResponse<Mailbox>>;
}

impl<T> JMAPSetMailbox<T> for JMAPStore<T>
where
    T: for<'x> Store<'x> + 'static,
{
    fn mailbox_set(&self, request: SetRequest<Mailbox>) -> jmap::Result<SetResponse<Mailbox>> {
        let mut helper = SetHelper::new(self, request)?;
        let on_destroy_remove_emails = helper
            .request
            .arguments
            .on_destroy_remove_emails
            .unwrap_or(false);

        helper.create(|_create_id, mailbox, helper, document| {
            // Set values
            let mut mailbox = TinyORM::<Mailbox>::new().mailbox_set(helper, mailbox, None, None)?;

            // Set parentId if the field is missing
            if !mailbox.has_property(&Property::ParentId) {
                mailbox.set(Property::ParentId, Value::Id { value: 0u64.into() });
            }
            mailbox.insert_validate(document)?;

            Ok((
                Mailbox::new(document.document_id.into()),
                None::<MutexGuard<'_, ()>>,
            ))
        })?;

        helper.update(|id, mailbox, helper, document| {
            let document_id = id.get_document_id();
            let current_fields = self
                .get_orm::<Mailbox>(helper.account_id, document_id)?
                .ok_or_else(|| SetError::new_err(SetErrorType::NotFound))?;
            let fields = TinyORM::track_changes(&current_fields).mailbox_set(
                helper,
                mailbox,
                document_id.into(),
                Some(&current_fields),
            )?;

            // Merge changes
            current_fields.merge_validate(document, fields)?;

            Ok(None)
        })?;

        helper.destroy(|id, helper, document| {
            let document_id = id.get_document_id();

            // Verify that this mailbox does not have sub-mailboxes
            if !self
                .query_store::<FilterMapper>(
                    helper.account_id,
                    Collection::Mailbox,
                    Filter::new_condition(
                        Property::ParentId.into(),
                        ComparisonOperator::Equal,
                        Query::LongInteger((document_id + 1) as LongInteger),
                    ),
                    Comparator::None,
                )?
                .is_empty()
            {
                return Err(SetError::new(
                    SetErrorType::MailboxHasChild,
                    "Mailbox has at least one children.",
                ));
            }

            // Verify that the mailbox is empty
            if let Some(message_doc_ids) = self.get_tag(
                helper.account_id,
                Collection::Mail,
                MessageField::Mailbox.into(),
                Tag::Id(document_id),
            )? {
                if on_destroy_remove_emails {
                    // Fetch results
                    for document_id in message_doc_ids {
                        let mut document = Document::new(Collection::Mail, document_id);
                        if let Some(id) = self.mail_delete(
                            helper.account_id,
                            Some(&mut helper.changes),
                            &mut document,
                        )? {
                            helper.changes.delete_document(document);
                            helper.changes.log_delete(Collection::Mail, id);
                        }
                    }
                } else {
                    return Err(SetError::new(
                        SetErrorType::MailboxHasEmail,
                        "Mailbox is not empty.",
                    ));
                }
            }

            // Delete ORM and index
            if let Some(orm) = helper
                .store
                .get_orm::<Mailbox>(helper.account_id, document_id)?
            {
                orm.delete(document);
            }

            Ok(())
        })?;

        helper.into_response()
    }
}

trait MailboxSet<T>: Sized
where
    T: for<'x> Store<'x> + 'static,
{
    fn mailbox_set(
        self,
        helper: &mut SetHelper<Mailbox, T>,
        mailbox: Mailbox,
        mailbox_id: Option<DocumentId>,
        fields: Option<&TinyORM<Mailbox>>,
    ) -> jmap::error::set::Result<Self, Property>;
}

impl<T> MailboxSet<T> for TinyORM<Mailbox>
where
    T: for<'x> Store<'x> + 'static,
{
    fn mailbox_set(
        mut self,
        helper: &mut SetHelper<Mailbox, T>,
        mailbox: Mailbox,
        mailbox_id: Option<DocumentId>,
        current_fields: Option<&TinyORM<Mailbox>>,
    ) -> jmap::error::set::Result<Self, Property> {
        //TODO implement isSubscribed
        // Set properties
        for (property, value) in mailbox.properties {
            let value = match (property, value) {
                (Property::Name, Value::Text { value }) => {
                    if value.len() < 255 {
                        Value::Text { value }
                    } else {
                        return Err(SetError::invalid_property(
                            property,
                            "Mailbox name is too long.".to_string(),
                        ));
                    }
                }
                (Property::ParentId, Value::Id { value }) => {
                    let parent_id = value.get_document_id();
                    if helper.will_destroy.contains(&value) {
                        return Err(SetError::new(
                            SetErrorType::WillDestroy,
                            "Parent ID will be destroyed.",
                        ));
                    } else if !helper.document_ids.contains(parent_id) {
                        return Err(SetError::new(
                            SetErrorType::InvalidProperties,
                            "Parent ID does not exist.",
                        ));
                    }

                    Value::Id {
                        value: (parent_id + 1).into(),
                    }
                }
                (Property::ParentId, Value::IdReference { value }) => Value::Id {
                    value: (u64::from(helper.get_id_reference(Property::ParentId, &value)?) + 1)
                        .into(),
                },
                (Property::ParentId, Value::Null) => Value::Id { value: 0u64.into() },
                (Property::Role, Value::Text { value }) => {
                    let role = value.to_lowercase();
                    if [
                        "inbox", "trash", "spam", "junk", "drafts", "archive", "sent",
                    ]
                    .contains(&role.as_str())
                    {
                        self.tag(property, Tag::Default);
                        Value::Text { value: role }
                    } else {
                        return Err(SetError::invalid_property(
                            property,
                            "Invalid role.".to_string(),
                        ));
                    }
                }
                (Property::Role, Value::Null) => {
                    self.untag(&property, &Tag::Default);
                    Value::Null
                }
                (Property::SortOrder, value @ Value::Number { .. }) => value,
                (_, _) => {
                    return Err(SetError::invalid_property(
                        property,
                        "Unexpected value.".to_string(),
                    ));
                }
            };

            self.set(property, value);
        }

        if let (Some(mailbox_id), Some(mut mailbox_parent_id)) = (
            mailbox_id,
            self.get(&Property::ParentId).and_then(|v| v.as_id()),
        ) {
            // Validate circular parent-child relationship
            let mut success = false;
            for _ in 0..helper.store.config.mailbox_max_depth {
                if mailbox_parent_id == (mailbox_id as store::JMAPId) + 1 {
                    return Err(SetError::new(
                        SetErrorType::InvalidProperties,
                        "Mailbox cannot be a parent of itself.",
                    ));
                } else if mailbox_parent_id == 0 {
                    success = true;
                    break;
                }

                mailbox_parent_id = helper
                    .store
                    .get_orm::<Mailbox>(
                        helper.account_id,
                        (mailbox_parent_id - 1).get_document_id(),
                    )?
                    .ok_or_else(|| StoreError::InternalError("Mailbox data not found".to_string()))?
                    .get(&Property::ParentId)
                    .and_then(|v| v.as_id())
                    .unwrap_or(0);
            }

            if !success {
                return Err(SetError::new(
                    SetErrorType::InvalidProperties,
                    "Mailbox parent-child relationship is too deep.",
                ));
            }
        }

        // Verify that the mailbox role is unique.
        if let Some(Value::Text {
            value: mailbox_role,
        }) = self.get(&Property::Role)
        {
            if !helper
                .store
                .query_store::<FilterMapper>(
                    helper.account_id,
                    Collection::Mailbox,
                    Filter::new_condition(
                        Property::Role.into(),
                        ComparisonOperator::Equal,
                        Query::Keyword(mailbox_role.into()),
                    ),
                    Comparator::None,
                )?
                .is_empty()
            {
                return Err(SetError::new(
                    SetErrorType::InvalidProperties,
                    format!("A mailbox with role '{}' already exists.", mailbox_role),
                ));
            }
        }

        // Verify that the mailbox name is unique.
        if let Some(Value::Text {
            value: mailbox_name,
        }) = self.get(&Property::Name)
        {
            // Obtain parent mailbox id
            if let Some(parent_mailbox_id) = if let Some(mailbox_parent_id) =
                &self.get(&Property::ParentId).and_then(|id| id.as_id())
            {
                (*mailbox_parent_id).into()
            } else if let Some(current_fields) = current_fields {
                if current_fields
                    .get(&Property::Name)
                    .and_then(|n| n.as_text())
                    != Some(mailbox_name)
                {
                    current_fields
                        .get(&Property::ParentId)
                        .and_then(|id| id.as_id())
                        .unwrap_or_default()
                        .into()
                } else {
                    None
                }
            } else {
                0.into()
            } {
                for jmap_id in helper.store.query_store::<FilterMapper>(
                    helper.account_id,
                    Collection::Mailbox,
                    Filter::new_condition(
                        Property::ParentId.into(),
                        ComparisonOperator::Equal,
                        Query::LongInteger(parent_mailbox_id),
                    ),
                    Comparator::None,
                )? {
                    if helper
                        .store
                        .get_orm::<Mailbox>(helper.account_id, jmap_id.get_document_id())?
                        .ok_or_else(|| {
                            StoreError::InternalError("Mailbox data not found".to_string())
                        })?
                        .get(&Property::Name)
                        .and_then(|n| n.as_text())
                        == Some(mailbox_name)
                    {
                        return Err(SetError::new(
                            SetErrorType::InvalidProperties,
                            format!("A mailbox with name '{}' already exists.", mailbox_name),
                        ));
                    }
                }
            }
        }

        Ok(self)
    }
}
