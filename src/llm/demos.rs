// Few-shot demos for LLM-guided POWL generation
//
// This module provides domain-specific examples that teach the LLM
// correct POWL modeling patterns. These are used by the JavaScript
// LLM layer to construct prompts for natural language to POWL generation.

/// Get few-shot demos for loan approval / finance domain
pub fn get_loan_approval_demos() -> String {
    r#"[
  {
    "description": "Simple loan approval with validation",
    "nl": "A customer submits a loan application. The application is validated. If valid, it is approved and paid. If invalid, it is rejected.",
    "powl": "X(PO=(nodes={Submit, Validate}, Approve->Pay, Reject}, order={Submit-->Validate, Validate-->X(Approve->Pay, Reject), Approve->Pay-->X(Approve->Pay, Reject)})"
  },
  {
    "description": "Loan approval with manual review",
    "nl": "A customer submits a loan application. It is validated automatically. If validation passes, it is approved. If validation fails, it goes to manual review. The reviewer can approve or reject.",
    "powl": "PO=(nodes={Submit, AutoValidate, X(ManualReview->Approve, ManualReview->Reject), Approve}, order={Submit-->AutoValidate, AutoValidate-->ManualReview, ManualReview-->X(Approve, Reject), X(Approve, Reject)-->Approve})"
  },
  {
    "description": "Loan with risk assessment",
    "nl": "A customer submits a loan application. The application is validated. Risk assessment is performed. If high risk, the application is rejected. If low risk, it is approved. After approval, payment is processed.",
    "powl": "PO=(nodes={Submit, Validate, RiskAssess, X(HighRisk->Reject, LowRisk->Approve->Pay)}, order={Submit-->Validate, Validate-->RiskAssess, RiskAssess-->X(HighRisk->Reject, LowRisk->Approve->Pay), X(HighRisk->Reject, LowRisk->Approve->Pay)-->Approve->Pay})"
  }
]"#.to_string()
}

/// Get few-shot demos for software release / IT domain
pub fn get_software_release_demos() -> String {
    r#"[
  {
    "description": "Simple software release process",
    "nl": "A developer commits code. The code is built. If the build succeeds, tests run. If tests pass, the release is deployed. If the build or tests fail, the process stops and the developer is notified.",
    "powl": "PO=(nodes={Commit, Build, Tests, Deploy, Notify}, order={Commit-->Build, Build-->Tests, Tests-->Deploy, Deploy-->Notify, Build-->Notify, Tests-->Notify})"
  },
  {
    "description": "CI/CD pipeline with staging",
    "nl": "Code is pushed to repository. CI builds the code. If build fails, notify developers. If build succeeds, run tests. If tests fail, notify developers. If tests pass, deploy to staging. If staging tests pass, deploy to production.",
    "powl": "PO=(nodes={Push, CI_Build, X(BuildFail->Notify, Tests), Staging, Production}, order={Push-->CI_Build, CI_Build-->X(BuildFail->Notify, Tests), Tests-->Staging, Staging-->Production, X(BuildFail->Notify, Tests)-->Notify})"
  },
  {
    "description": "Release with rollback option",
    "nl": "Code is deployed to production. Monitoring runs for 30 minutes. If issues are detected, rollback is triggered. If no issues, the release is complete.",
    "powl": "PO=(nodes={Deploy, Monitor, X(Rollback->Notify, Complete)}, order={Deploy-->Monitor, Monitor-->X(Rollback->Notify, Complete)})"
  }
]"#.to_string()
}

/// Get few-shot demos for e-commerce / retail domain
pub fn get_ecommerce_demos() -> String {
    r#"[
  {
    "description": "Order fulfillment",
    "nl": "A customer places an order. The order is confirmed. Payment is processed. If payment succeeds, the order is picked and shipped. If payment fails, the order is cancelled.",
    "powl": "PO=(nodes={PlaceOrder, Confirm, X(PaySuccess->PickAndShip, PayFail->Cancel), PickAndShip}, order={PlaceOrder-->Confirm, Confirm-->X(PaySuccess->PickAndShip, PayFail->Cancel), X(PaySuccess->PickAndShip, PayFail->Cancel)-->PickAndShip})"
  },
  {
    "description": "Order with multiple payment methods",
    "nl": "Customer places order. They can pay with credit card or PayPal. After payment, order is processed and shipped.",
    "powl": "X(CreditCard->Process, PayPal->Process)->Process->Ship"
  },
  {
    "description": "Return processing",
    "nl": "Customer requests return. Return is received. If item is in good condition, refund is processed. If item is damaged, return is rejected.",
    "powl": "RequestReturn->ReceiveReturn->X(GoodCondition->Refund, Damaged->Reject)"
  }
]"#.to_string()
}

/// Get few-shot demos for manufacturing / production domain
pub fn get_manufacturing_demos() -> String {
    r#"[
  {
    "description": "Production line with quality check",
    "nl": "Raw material enters production. It is processed. Quality check is performed. If quality passes, item is packaged. If quality fails, item is reworked. After rework, quality check is repeated.",
    "powl": "Process->QualityCheck->X(Pass->Package, Fail->Rework)->Package"
  },
  {
    "description": "Multi-stage manufacturing",
    "nl": "Material goes through Stage 1, then Stage 2, then Stage 3 in sequence. After Stage 3, final inspection occurs. Approved items go to shipping. Defective items go to scrap.",
    "powl": "Stage1->Stage2->Stage3->Inspection->X(Approve->Ship, Defect->Scrap)"
  },
  {
    "description": "Maintenance workflow",
    "nl": "Machine is monitored. If anomaly detected, maintenance is triggered. Maintenance can be preventive or corrective. After maintenance, machine resumes normal operation.",
    "powl": "Monitor->X(Anomaly->Maintenance, Normal)->X(Preventive, Corrective)->Resume"
  }
]"#.to_string()
}

/// Get few-shot demos for healthcare / medical domain
pub fn get_healthcare_demos() -> String {
    r#"[
  {
    "description": "Patient admission",
    "nl": "Patient arrives at hospital. Registration is completed. Triage assesses severity. If emergency, patient goes to emergency room immediately. If non-emergency, patient waits for consultation. After consultation, patient is either discharged or admitted.",
    "powl": "Registration->Triage->X(Emergency->ER, NonEmergency->Wait)->Consultation->X(Discharge, Admit)"
  },
  {
    "description": "Medication administration",
    "nl": "Nurse prepares medication. Patient is identified. Medication is administered. Response is monitored. If adverse reaction, treatment is given. If no reaction, monitoring continues.",
    "powl": "Prepare->Identify->Administer->Monitor->X(AdverseReaction->Treat, NoReaction->Continue)"
  },
  {
    "description": "Diagnostic workflow",
    "nl": "Doctor orders diagnostic test. Test is performed. Results are reviewed. If abnormal, specialist consultation is scheduled. If normal, results are communicated to patient.",
    "powl": "OrderTest->PerformTest->ReviewResults->X(Abnormal->Specialist, Normal->Communicate)"
  }
]"#.to_string()
}

/// Get general few-shot demos (domain-agnostic)
pub fn get_general_demos() -> String {
    r#"[
  {
    "description": "Simple sequence",
    "nl": "Do task A, then task B, then task C.",
    "powl": "A->B->C"
  },
  {
    "description": "Choice between alternatives",
    "nl": "Process starts with task A. Then either task B or task C is performed. Finally, task D completes the process.",
    "powl": "A->X(B, C)->D"
  },
  {
    "description": "Parallel execution",
    "nl": "Tasks A and B can happen at the same time. After both complete, task C is performed.",
    "powl": "PO=(nodes={A, B, C}, order={A-->C, B-->C})"
  },
  {
    "description": "Loop with option to skip",
    "nl": "Perform task A, then optionally repeat task B. You can choose to exit after any iteration.",
    "powl": "*(A, B)"
  },
  {
    "description": "Optional task",
    "nl": "Task A is performed. Task B is optional - it may be skipped. Then task C completes.",
    "powl": "A->X(B, tau)->C"
  }
]"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_demos() {
        let demos = get_loan_approval_demos();
        assert!(demos.contains("loan_approval"));

        let demos = get_software_release_demos();
        assert!(demos.contains("software_release"));

        let demos = get_manufacturing_demos();
        assert!(demos.contains("manufacturing"));
    }
}
