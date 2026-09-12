#[cfg(not(tarpaulin_include))]
use crate::account::{Account, AccountKind};
use crate::{client::transfer::error::TransferError, client::BoursoWebClient, constants::BASE_URL};
use anyhow::{bail, Context, Result};
use futures_util::stream::Stream;
use tracing::debug;

mod error;

/// `Characteristics[paymentType]` value for an instant transfer, credited in seconds.
/// The other value the form offers is "1", a classic transfer settling in 1 to 3
/// business days.
const PAYMENT_TYPE_INSTANT: &str = "0";

#[derive(Debug, Clone)]
pub enum TransferProgress {
    Validating,
    InitializingTransfer,
    ExtractingFlowInstance,
    SettingDebitAccount,
    SettingCreditAccount,
    SettingAmount,
    AcknowledgingPayeeVerification,
    SettingReason,
    SubmittingRecap,
    ConfirmingTransfer,
    Completed,
}

impl TransferProgress {
    #[cfg(not(tarpaulin_include))]
    pub fn step_number(&self) -> u8 {
        match self {
            TransferProgress::Validating => 1,
            TransferProgress::InitializingTransfer => 2,
            TransferProgress::ExtractingFlowInstance => 3,
            TransferProgress::SettingDebitAccount => 4,
            TransferProgress::SettingCreditAccount => 5,
            TransferProgress::SettingAmount => 6,
            TransferProgress::AcknowledgingPayeeVerification => 7,
            TransferProgress::SettingReason => 8,
            TransferProgress::SubmittingRecap => 9,
            TransferProgress::ConfirmingTransfer => 10,
            TransferProgress::Completed => 11,
        }
    }

    pub fn total_steps() -> u8 {
        11
    }

    #[cfg(not(tarpaulin_include))]
    pub fn description(&self) -> &str {
        match self {
            TransferProgress::Validating => "Validating transfer parameters",
            TransferProgress::InitializingTransfer => "Initializing transfer",
            TransferProgress::ExtractingFlowInstance => "Extracting flow instance",
            TransferProgress::SettingDebitAccount => "Setting debit account",
            TransferProgress::SettingCreditAccount => "Setting credit account",
            TransferProgress::SettingAmount => "Setting transfer amount",
            TransferProgress::AcknowledgingPayeeVerification => {
                "Acknowledging the payee verification"
            }
            TransferProgress::SettingReason => "Setting transfer reason",
            TransferProgress::SubmittingRecap => "Submitting the recap",
            TransferProgress::ConfirmingTransfer => "Confirming transfer",
            TransferProgress::Completed => "Transfer completed",
        }
    }
}

impl BoursoWebClient {
    /// Initialize the transfer and extract the transfer ID
    #[cfg(not(tarpaulin_include))]
    async fn init_transfer(&self, from_account: &str) -> Result<String> {
        let init_transfer_url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau",
            BASE_URL, from_account
        );

        let res = self.client.get(&init_transfer_url).send().await?;

        if res.status() != 302 {
            debug!("Init transfer response: {:?}", res);
            bail!(TransferError::TransferInitiationFailed);
        }

        let location = res
            .headers()
            .get("location")
            .context("Missing Location header")?
            .to_str()?;

        // /compte/cav/XXXXXXX/virements/immediat/nouveau/YYYYY/1
        // get YYYYY
        let transfer_id = location
            .split('/')
            .nth(7)
            .context("Failed to extract transfer id")?
            .to_string();

        Ok(transfer_id)
    }

    /// Extract the flow instance from the HTML response
    #[cfg(not(tarpaulin_include))]
    async fn extract_flow_instance(&self, url: &str) -> Result<String> {
        let res = self.client.get(url).send().await?;

        if res.status() != 200 {
            debug!("First transfer step response: {:?}", res);
            bail!(TransferError::TransferInitiationFailed);
        }

        let res_text = res.text().await?;
        let re = regex::Regex::new(r#"name="flow_ImmediateCashTransfer_instance" value="([^"]+)""#)
            .unwrap();
        let flow_instance = re
            .captures(&res_text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .context("Failed to extract flow instance")?
            .to_string();

        Ok(flow_instance)
    }

    /// Set the debit account (step 2)
    #[cfg(not(tarpaulin_include))]
    async fn set_debit_account(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "1".to_string())
            .text("DebitAccount[debit]", from_account.to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/2",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Set debit account response: {}", body);
            bail!(TransferError::SetDebitAccountFailed);
        }

        log_flow_step("set debit account", &body);

        Ok(())
    }

    /// Set the credit account (step 3)
    #[cfg(not(tarpaulin_include))]
    async fn set_credit_account(
        &self,
        from_account: &str,
        to_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        _transfer_from_banking: bool,
    ) -> Result<()> {
        // The form only exposes CreditAccount[credit]; the CreditAccount[newBeneficiary]
        // field it used to carry is gone, and Symfony rejects a form that carries an
        // extra field, which stalled this step on the account picker.
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "2".to_string())
            .text("CreditAccount[credit]", to_account.to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/3",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Set credit account response: {}", body);
            bail!(TransferError::SetCreditAccountFailed);
        }

        log_flow_step("set credit account", &body);

        Ok(())
    }

    /// Set the transfer amount (step 4)
    #[cfg(not(tarpaulin_include))]
    async fn set_transfer_amount(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        amount: f64,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "3".to_string())
            .text("Amount[amount]", format!("{:.2}", amount).replace('.', ","))
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/4",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Set amount response: {}", body);
            bail!(TransferError::SetAmountFailed);
        }

        log_flow_step("set transfer amount", &body);

        Ok(())
    }

    /// Acknowledge the Verification of Payee result (step 4)
    ///
    /// SEPA payee verification became a mandatory step of the flow; the form carries
    /// no field of its own, acknowledging it is just a submit.
    #[cfg(not(tarpaulin_include))]
    async fn submit_step_5(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "4".to_string())
            .text("submit", "".to_string());

        let res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/5",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Submit transfer response: {}", body);
            bail!(TransferError::Step5Failed);
        }

        log_flow_step("submit step 5", &body);

        Ok(())
    }

    /// Set the transfer reason and payment type (step 7)
    ///
    /// The form asks for `Characteristics[paymentType]`, which is required; the
    /// `Characteristics[schedulingType]` field this used to send no longer exists.
    #[cfg(not(tarpaulin_include))]
    async fn set_transfer_reason(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        transfer_reason: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "6".to_string())
            .text("Characteristics[label]", transfer_reason.to_string())
            .text("Characteristics[paymentType]", PAYMENT_TYPE_INSTANT.to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/7",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Set reason response: {}", body);
            bail!(TransferError::SetReasonFailed);
        }

        log_flow_step("set transfer reason", &body);

        Ok(())
    }

    /// Validate the recap screen (step 8)
    ///
    /// Between the characteristics and the final confirmation the flow shows a recap
    /// with a single "Valider" button. It carries no field of its own.
    ///
    /// # Returns
    ///
    /// `true` when this submission already reached the confirmation screen, which is
    /// what the flow does today: validating the recap executes the transfer.
    #[cfg(not(tarpaulin_include))]
    async fn submit_recap(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<bool> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "8".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/9",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Submit recap response: {}", body);
            bail!(TransferError::SubmitTransferFailed);
        }

        log_flow_step("submit recap", &body);

        Ok(is_confirmation_page(&body))
    }

    /// Confirm and finalize the transfer (step 9)
    #[cfg(not(tarpaulin_include))]
    async fn confirm_transfer(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "9".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/10",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if status != 200 {
            debug!("Confirm transfer response: {}", body);
            bail!(TransferError::SubmitTransferFailed);
        }

        if is_confirmation_page(&body) {
            return Ok(());
        }

        // A rejected submission comes back as the same form re-rendered, with a 200
        // status. Reporting which step the server is still on says far more than a
        // missing confirmation message.
        log_flow_step("confirm transfer", &body);

        // This is the failure path, so the page is always worth keeping whatever the
        // debug flag says: it is the only record of why the transfer was refused.
        debug!("Transfer not confirmed, full response: {}", body);

        bail!(TransferError::InvalidTransfer);
    }

    /// Transfer funds from one account to another, yielding progress updates
    ///
    /// ## Arguments
    /// - `amount`: Amount to transfer (must be >= 10.0)
    /// - `from_account`: Source account
    /// - `to_account`: Destination account
    /// - `reason`: Optional reason for the transfer (max 50 characters)
    ///
    /// ## Returns
    /// A stream of progress updates for the transfer.
    #[cfg(not(tarpaulin_include))]
    pub fn transfer_funds(
        &self,
        amount: f64,
        from_account: Account,
        to_account: Account,
        reason: Option<String>,
    ) -> impl Stream<Item = Result<TransferProgress>> + '_ {
        async_stream::stream! {
            // Validation
            yield Ok(TransferProgress::Validating);

            if amount < 10.0 {
                yield Err(TransferError::AmountTooLow.into());
                return;
            }

            debug!(
                "Initiating transfer of {:.2} EUR from account {} to account {}",
                amount,
                from_account.id,
                to_account.id
            );

            let transfer_from_banking = from_account.kind == AccountKind::Banking;
            let from_account_id = from_account.id.clone();
            let to_account_id = to_account.id.clone();

            // Default reason if none provided, else use provided reason and
            // warn if the reason is too long (> 50 characters)
            let transfer_reason = if let Some(r) = reason {
                if r.len() > 50 {
                    yield Err(TransferError::ReasonIsTooLong.into());
                    return;
                }
                r
            } else {
                "Virement depuis BoursoBank".to_string()
            };

            // Step 1: Initialize transfer and get transfer ID
            yield Ok(TransferProgress::InitializingTransfer);
            let transfer_id = match self.init_transfer(&from_account_id).await {
                Ok(id) => id,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Extract flow instance
            yield Ok(TransferProgress::ExtractingFlowInstance);
            let flow_instance = match self
                .extract_flow_instance(&format!(
                    "{}/compte/cav/{}/virements/immediat/nouveau/{}/1",
                    BASE_URL, &from_account_id, transfer_id
                ))
                .await {
                Ok(flow) => flow,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Step 2: Set debit account
            yield Ok(TransferProgress::SettingDebitAccount);
            if let Err(e) = self.set_debit_account(&from_account_id, &transfer_id, &flow_instance)
                .await {
                yield Err(e);
                return;
            }

            // Step 3: Set credit account
            yield Ok(TransferProgress::SettingCreditAccount);
            if let Err(e) = self.set_credit_account(
                &from_account_id,
                &to_account_id,
                &transfer_id,
                &flow_instance,
                transfer_from_banking,
            )
            .await {
                yield Err(e);
                return;
            }

            // Step 6: Set amount
            yield Ok(TransferProgress::SettingAmount);
            if let Err(e) = self.set_transfer_amount(&from_account_id, &transfer_id, &flow_instance, amount)
                .await {
                yield Err(e);
                return;
            }

            // Step 4: Acknowledge the SEPA payee verification
            yield Ok(TransferProgress::AcknowledgingPayeeVerification);
            if let Err(e) = self.submit_step_5(&from_account_id, &transfer_id, &flow_instance)
                .await {
                yield Err(e);
                return;
            }

            // Step 6: Set reason and payment type
            yield Ok(TransferProgress::SettingReason);
            if let Err(e) = self.set_transfer_reason(
                &from_account_id,
                &transfer_id,
                &flow_instance,
                &transfer_reason,
            )
            .await {
                yield Err(e);
                return;
            }

            // Step 8: Validate the recap. This is what executes the transfer today, so
            // it can land straight on the confirmation screen.
            yield Ok(TransferProgress::SubmittingRecap);
            let already_confirmed = match self.submit_recap(&from_account_id, &transfer_id, &flow_instance)
                .await {
                Ok(confirmed) => confirmed,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Only submit the final step when the recap did not already confirm:
            // posting it again on a finished flow answers with no confirmation at all,
            // which used to be reported as a failed transfer that had in fact gone through.
            if !already_confirmed {
                yield Ok(TransferProgress::ConfirmingTransfer);
                if let Err(e) = self.confirm_transfer(&from_account_id, &transfer_id, &flow_instance)
                    .await {
                    yield Err(e);
                    return;
                }
            }

            yield Ok(TransferProgress::Completed);
        }
    }
}

/// Whether a response is the confirmation screen that ends the flow.
///
/// Deliberately conservative: it requires both that the flow form is gone and that a
/// success marker is present. A re-rendered form is never a success, and mistaking a
/// refusal for a confirmation is far worse than the opposite.
fn is_confirmation_page(body: &str) -> bool {
    extract_flow_step(body).is_none()
        && (body.contains("c-alert--success") || body.contains(">Confirmation</h3>"))
}

/// Extract the step number the server reports in the flow form it returns.
///
/// # Arguments
///
/// * `body` - The HTML response of a transfer step.
///
/// # Returns
///
/// The step number as a string, or `None` when the response carries no flow form.
fn extract_flow_step(body: &str) -> Option<String> {
    let re = regex::Regex::new(r#"name="flow_ImmediateCashTransfer_step"\s*value="(?P<step>\d+)""#)
        .unwrap();

    re.captures(body)
        .and_then(|captures| captures.name("step"))
        .map(|step| step.as_str().to_string())
}

/// Log the step the server reports after a submission.
///
/// Each step only checks the HTTP status, but a submission the server rejects is
/// re-rendered with status 200, so the flow can silently stall on a step instead of
/// advancing. Logging the reported step makes that visible in `~/.bourso/bourso.log`.
#[cfg(not(tarpaulin_include))]
fn log_flow_step(label: &str, body: &str) {
    match extract_flow_step(body) {
        Some(step) => debug!("After '{}', the server reports flow step {}", label, step),
        None => debug!("After '{}', the response carries no flow step", label),
    }

    // The step number says that a submission was refused, never why. Set
    // BOURSO_DEBUG_TRANSFER=1 to also capture each page in ~/.bourso/bourso.log,
    // which is what it takes to see the fields the form actually expects.
    if std::env::var("BOURSO_DEBUG_TRANSFER").is_ok() {
        debug!("Body after '{}': {}", label, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_flow_step() {
        let body = r#"<input  id="form_flow_ImmediateCashTransfer_step" type="hidden" class="c-field__input" name="flow_ImmediateCashTransfer_step" value="9" >"#;

        assert_eq!(extract_flow_step(body), Some("9".to_string()));
    }

    #[test]
    fn test_extract_flow_step_absent() {
        assert_eq!(extract_flow_step("<html><body>Confirmation</body></html>"), None);
    }
}
