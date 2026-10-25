<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-1034 REQ-1582 -->
# NIST SP 800-53 Revision 5 security and privacy controls

## How to read this file

This file holds the requirements of one subject, kept by people and agents together. Leave its first two lines as they are.

- Under `Requirements` each bullet is an item (`ITEM-`), a part of the subject, or a requirement (`REQ-`), which states one requirement. An item holds requirements and further items, indented under it. A requirement stands in an item and holds nothing.
- A number is never changed and never used again. Add an entry with `ITEM-NEW` or `REQ-NEW` in place of a number: the designer numbers it.
- Lines indented under a bullet that are not bullets describe it. A description holds no bullets.
- A bullet's first line ends with its tags (`#name`), and a requirement's with one category (`category:name`) and one state (`state:name`) after them. A name has no spaces. A title does not end in a word that starts with `#`, `category:` or `state:`.
- The categories are functional, non-functional and technical, and the states proposed, accepted, rejected and done; others can be added.
- Refer to an entry by its number in square brackets: `[REQ-0001]`.
- Under `Assessments` each bullet is a question, remark, warning or idea about one entry, named by its number: `- [ ] Question REQ-0001: text`. Write `[x]` to resolve one. Lines indented under it are its detail.

```markdown
- ITEM-NEW Checkout #core
  What a customer does between the basket and the receipt.
  - REQ-NEW An order is confirmed by e-mail. #mail category:functional state:proposed
    Sent once the payment of [REQ-0001] succeeds.

- [ ] Question REQ-0001: Which payment methods count?
- [x] Idea ITEM-0001: Offer a guest checkout.
```

## Requirements

The controls of NIST SP 800-53 Revision 5 (release 5.2.0), one item per family, control and control enhancement, and one requirement per part of a control's statement. A requirement is tagged with the baselines of NIST SP 800-53B that select its control.

- ITEM-0001 Access Control
  The access control family (AC) of NIST SP 800-53 Revision 5.
  - ITEM-0002 AC-1 Policy and Procedures
    Related controls: [ITEM-0340], [ITEM-0565], [ITEM-0582], [ITEM-0608], [ITEM-0972].
    - REQ-0001 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] access control policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the access control policy and the associated access controls;
    - REQ-0002 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the access control policy and procedures; and #low #moderate #high #privacy
    - REQ-0003 (c) Review and update the current access control: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0003 AC-2 Account Management
    Related controls: [ITEM-0016], [ITEM-0061], [ITEM-0062], [ITEM-0102], [ITEM-0110], [ITEM-0118], [ITEM-0129], [ITEM-0151], [ITEM-0190], [ITEM-0251], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0379], [ITEM-0444], [ITEM-0458], [ITEM-0492], [ITEM-0545], [ITEM-0594], [ITEM-0600], [ITEM-0603], [ITEM-0607], [ITEM-0612], [ITEM-0615], [ITEM-0782], [ITEM-0818], [ITEM-0823], [ITEM-0875].
    - REQ-0004 (a) Define and document the types of accounts allowed and specifically prohibited for use within the system; #low #moderate #high
    - REQ-0005 (b) Assign account managers; #low #moderate #high
    - REQ-0006 (c) Require [Assignment: organization-defined prerequisites and criteria] for group and role membership; #low #moderate #high
    - REQ-0007 (d) Specify: #low #moderate #high
      (d)(1) Authorized users of the system;
      (d)(2) Group and role membership; and
      (d)(3) Access authorizations (i.e., privileges) and [Assignment: organization-defined attributes (as required)] for each account;
    - REQ-0008 (e) Require approvals by [Assignment: organization-defined personnel or roles] for requests to create accounts; #low #moderate #high
    - REQ-0009 (f) Create, enable, modify, disable, and remove accounts in accordance with [Assignment: organization-defined policy, procedures, prerequisites, and criteria]; #low #moderate #high
    - REQ-0010 (g) Monitor the use of accounts; #low #moderate #high
    - REQ-0011 (h) Notify account managers and [Assignment: organization-defined personnel or roles] within: #low #moderate #high
      (h)(1) [Assignment: organization-defined time period] when accounts are no longer required;
      (h)(2) [Assignment: organization-defined time period] when users are terminated or transferred; and
      (h)(3) [Assignment: organization-defined time period] when system usage or need-to-know changes for an individual;
    - REQ-0012 (i) Authorize access to the system based on: #low #moderate #high
      (i)(1) A valid access authorization;
      (i)(2) Intended system usage; and
      (i)(3) [Assignment: organization-defined attributes (as required)];
    - REQ-0013 (j) Review accounts for compliance with account management requirements [Assignment: organization-defined frequency]; #low #moderate #high
    - REQ-0014 (k) Establish and implement a process for changing shared or group account authenticators (if deployed) when individuals are removed from the group; and #low #moderate #high
    - REQ-0015 (l) Align account management processes with personnel termination and transfer processes. #low #moderate #high
    - ITEM-0004 AC-2(1) Automated System Account Management
      - REQ-0016 Support the management of system accounts using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0005 AC-2(2) Automated Temporary and Emergency Account Management
      - REQ-0017 Automatically [Selection: remove; disable] temporary and emergency accounts after [Assignment: organization-defined time period]. #moderate #high
    - ITEM-0006 AC-2(3) Disable Accounts
      - REQ-0018 (a) Have expired; #moderate #high
      - REQ-0019 (b) Are no longer associated with a user or individual; #moderate #high
      - REQ-0020 (c) Are in violation of organizational policy; or #moderate #high
      - REQ-0021 (d) Have been inactive for [Assignment: organization-defined time period]. #moderate #high
    - ITEM-0007 AC-2(4) Automated Audit Actions
      Related controls: [ITEM-0151], [ITEM-0163].
      - REQ-0022 Automatically audit account creation, modification, enabling, disabling, and removal actions. #moderate #high
    - ITEM-0008 AC-2(5) Inactivity Logout
      Related controls: [ITEM-0084].
      - REQ-0023 Require that users log out when [Assignment: organization-defined time period of expected inactivity or description of when to log out]. #moderate #high
    - ITEM-0009 AC-2(6) Dynamic Privilege Management
      Related controls: [ITEM-0091].
      - REQ-0024 Implement [Assignment: organization-defined dynamic privilege management capabilities].
    - ITEM-0010 AC-2(7) Privileged User Accounts
      - REQ-0025 (a) Establish and administer privileged user accounts in accordance with [Selection: a role-based access scheme; an attribute-based access scheme];
      - REQ-0026 (b) Monitor privileged role or attribute assignments;
      - REQ-0027 (c) Monitor changes to roles or attributes; and
      - REQ-0028 (d) Revoke access when privileged role or attribute assignments are no longer appropriate.
    - ITEM-0011 AC-2(8) Dynamic Account Management
      Related controls: [ITEM-0091].
      - REQ-0029 Create, activate, manage, and deactivate [Assignment: organization-defined system accounts] dynamically.
    - ITEM-0012 AC-2(9) Restrictions on Use of Shared and Group Accounts
      - REQ-0030 Only permit the use of shared and group accounts that meet [Assignment: organization-defined conditions].
    - ITEM-0013 AC-2(11) Usage Conditions
      - REQ-0031 Enforce [Assignment: organization-defined circumstances and/or usage conditions] for [Assignment: organization-defined system accounts]. #high
    - ITEM-0014 AC-2(12) Account Monitoring for Atypical Usage
      Related controls: [ITEM-0163], [ITEM-0172], [ITEM-0220], [ITEM-0434], [ITEM-0918].
      - REQ-0032 (a) Monitor system accounts for [Assignment: organization-defined atypical usage] ; and #high
      - REQ-0033 (b) Report atypical usage of system accounts to [Assignment: organization-defined personnel or roles]. #high
    - ITEM-0015 AC-2(13) Disable Accounts for High-risk Individuals
      Related controls: [ITEM-0163], [ITEM-0918].
      - REQ-0034 Disable accounts of individuals within [Assignment: organization-defined time period] of discovery of [Assignment: organization-defined significant risks]. #moderate #high
  - ITEM-0016 AC-3 Access Enforcement
    Related controls: [ITEM-0003], [ITEM-0030], [ITEM-0061], [ITEM-0062], [ITEM-0091], [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0118], [ITEM-0124], [ITEM-0127], [ITEM-0129], [ITEM-0132], [ITEM-0135], [ITEM-0142], [ITEM-0175], [ITEM-0230], [ITEM-0251], [ITEM-0282], [ITEM-0341], [ITEM-0361], [ITEM-0377], [ITEM-0378], [ITEM-0387], [ITEM-0395], [ITEM-0444], [ITEM-0451], [ITEM-0458], [ITEM-0473], [ITEM-0556], [ITEM-0595], [ITEM-0612], [ITEM-0615], [ITEM-0749], [ITEM-0766], [ITEM-0769], [ITEM-0775], [ITEM-0818], [ITEM-0823], [ITEM-0851], [ITEM-0862], [ITEM-0868], [ITEM-0918], [ITEM-0961].
    - REQ-0035 Enforce approved authorizations for logical access to information and system resources in accordance with applicable access control policies. #low #moderate #high
    - ITEM-0017 AC-3(2) Dual Authorization
      Related controls: [ITEM-0324], [ITEM-0477].
      - REQ-0036 Enforce dual authorization for [Assignment: organization-defined privileged commands and/or other actions].
    - ITEM-0018 AC-3(3) Mandatory Access Control
      Related controls: [ITEM-0782].
      - REQ-0037 (a) Is uniformly enforced across the covered subjects and objects within the system;
      - REQ-0038 (b) Specifies that a subject that has been granted access to information is constrained from doing any of the following;
        (b)(1) Passing the information to unauthorized subjects or objects;
        (b)(2) Granting its privileges to other subjects;
        (b)(3) Changing one or more security attributes (specified by the policy) on subjects, objects, the system, or system components;
        (b)(4) Choosing the security attributes and attribute values (specified by the policy) to be associated with newly created or modified objects; and
        (b)(5) Changing the rules governing access control; and
      - REQ-0039 (c) Specifies that [Assignment: organization-defined subjects] may explicitly be granted [Assignment: organization-defined privileges] such that they are not limited by any defined subset (or all) of the above constraints.
    - ITEM-0019 AC-3(4) Discretionary Access Control
      - REQ-0040 (a) Pass the information to any other subjects or objects;
      - REQ-0041 (b) Grant its privileges to other subjects;
      - REQ-0042 (c) Change security attributes on subjects, objects, the system, or the system’s components;
      - REQ-0043 (d) Choose the security attributes to be associated with newly created or revised objects; or
      - REQ-0044 (e) Change the rules governing access control.
    - ITEM-0020 AC-3(5) Security-relevant Information
      Related controls: [ITEM-0256], [ITEM-0878].
      - REQ-0045 Prevent access to [Assignment: organization-defined security-relevant information] except during secure, non-operable system states.
    - ITEM-0021 AC-3(7) Role-based Access Control
      - REQ-0046 Enforce a role-based access control policy over defined subjects and objects and control access based upon [Assignment: organization-defined roles and users authorized to assume such roles].
    - ITEM-0022 AC-3(8) Revocation of Access Authorizations
      - REQ-0047 Enforce the revocation of access authorizations resulting from changes to the security attributes of subjects and objects based on [Assignment: organization-defined rules].
    - ITEM-0023 AC-3(9) Controlled Release
      Related controls: [ITEM-0212], [ITEM-0628], [ITEM-0631], [ITEM-0709], [ITEM-0828].
      - REQ-0048 (a) The receiving [Assignment: organization-defined system or system component] provides [Assignment: organization-defined controls] ; and
      - REQ-0049 (b) [Assignment: organization-defined controls] are used to validate the appropriateness of the information designated for release.
    - ITEM-0024 AC-3(10) Audited Override of Access Control Mechanisms
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0183], [ITEM-0190], [ITEM-0199].
      - REQ-0050 Employ an audited override of automated access control mechanisms under [Assignment: organization-defined conditions] by [Assignment: organization-defined roles].
    - ITEM-0025 AC-3(11) Restrict Access to Specific Information Types
      Related controls: [ITEM-0269], [ITEM-0285], [ITEM-0287], [ITEM-0559].
      - REQ-0051 Restrict access to data repositories containing [Assignment: organization-defined information types].
    - ITEM-0026 AC-3(12) Assert and Enforce Application Access
      Related controls: [ITEM-0259].
      - REQ-0052 (a) Require applications to assert, as part of the installation process, the access needed to the following system applications and functions: [Assignment: organization-defined system applications and functions];
      - REQ-0053 (b) Provide an enforcement mechanism to prevent unauthorized access; and
      - REQ-0054 (c) Approve access changes after initial installation of the application.
    - ITEM-0027 AC-3(13) Attribute-based Access Control
      - REQ-0055 Enforce attribute-based access control policy over defined subjects and objects and control access based upon [Assignment: organization-defined attributes].
    - ITEM-0028 AC-3(14) Individual Access
      Related controls: [ITEM-0379], [ITEM-0580], [ITEM-0577], [ITEM-0579], [ITEM-0625].
      - REQ-0056 Provide [Assignment: organization-defined mechanisms] to enable individuals to have access to the following elements of their personally identifiable information: [Assignment: organization-defined elements]. #privacy
    - ITEM-0029 AC-3(15) Discretionary and Mandatory Access Control
      Related controls: [ITEM-0766], [ITEM-0769], [ITEM-0030].
      - REQ-0057 (a) Enforce [Assignment: organization-defined mandatory access control policy] over the set of covered subjects and objects specified in the policy; and
      - REQ-0058 (b) Enforce [Assignment: organization-defined discretionary access control policy] over the set of covered subjects and objects specified in the policy.
  - ITEM-0030 AC-4 Information Flow Enforcement
    Related controls: [ITEM-0016], [ITEM-0062], [ITEM-0091], [ITEM-0102], [ITEM-0115], [ITEM-0124], [ITEM-0183], [ITEM-0212], [ITEM-0230], [ITEM-0259], [ITEM-0551], [ITEM-0582], [ITEM-0749], [ITEM-0775], [ITEM-0782], [ITEM-0828], [ITEM-0862].
    - REQ-0059 Enforce approved authorizations for controlling the flow of information within the system and between connected systems based on [Assignment: organization-defined information flow control policies]. #moderate #high
    - ITEM-0031 AC-4(1) Object Security and Privacy Attributes
      - REQ-0060 Use [Assignment: organization-defined security and privacy attributes] associated with [Assignment: organization-defined information, source, and destination objects] to enforce [Assignment: organization-defined information flow control policies] as a basis for flow control decisions.
    - ITEM-0032 AC-4(2) Processing Domains
      Related controls: [ITEM-0878].
      - REQ-0061 Use protected processing domains to enforce [Assignment: organization-defined information flow control policies] as a basis for flow control decisions.
    - ITEM-0033 AC-4(3) Dynamic Information Flow Control
      Related controls: [ITEM-0918].
      - REQ-0062 Enforce [Assignment: organization-defined information flow control policies].
    - ITEM-0034 AC-4(4) Flow Control of Encrypted Information
      Related controls: [ITEM-0918].
      - REQ-0063 Prevent encrypted information from bypassing [Assignment: organization-defined information flow control mechanisms] by [Selection (one or more): decrypting the information; blocking the flow of the encrypted information; terminating communications sessions attempting to pass encrypted information; [Assignment: organization-defined procedure or method] ]. #high
    - ITEM-0035 AC-4(5) Embedded Data Types
      - REQ-0064 Enforce [Assignment: organization-defined limitations] on embedding data types within other data types.
    - ITEM-0036 AC-4(6) Metadata
      Related controls: [ITEM-0091], [ITEM-0947].
      - REQ-0065 Enforce information flow control based on [Assignment: organization-defined metadata].
    - ITEM-0037 AC-4(7) One-way Flow Mechanisms
      - REQ-0066 Enforce one-way information flows through hardware-based flow control mechanisms.
    - ITEM-0038 AC-4(8) Security and Privacy Policy Filters
      - REQ-0067 (a) Enforce information flow control using [Assignment: organization-defined security or privacy policy filters] as a basis for flow control decisions for [Assignment: organization-defined information flows] ; and
      - REQ-0068 (b) [Selection (one or more): block; strip; modify; quarantine] data after a filter processing failure in accordance with [Assignment: organization-defined security or privacy policy].
    - ITEM-0039 AC-4(9) Human Reviews
      - REQ-0069 Enforce the use of human reviews for [Assignment: organization-defined information flows] under the following conditions: [Assignment: organization-defined conditions].
    - ITEM-0040 AC-4(10) Enable and Disable Security or Privacy Policy Filters
      - REQ-0070 Provide the capability for privileged administrators to enable and disable [Assignment: organization-defined security or privacy policy filters] under the following conditions: [Assignment: organization-defined conditions].
    - ITEM-0041 AC-4(11) Configuration of Security or Privacy Policy Filters
      - REQ-0071 Provide the capability for privileged administrators to configure [Assignment: organization-defined security or privacy policy filters] to support different security or privacy policies.
    - ITEM-0042 AC-4(12) Data Type Identifiers
      - REQ-0072 When transferring information between different security domains, use [Assignment: organization-defined data type identifiers] to validate data essential for information flow decisions.
    - ITEM-0043 AC-4(13) Decomposition into Policy-relevant Subcomponents
      - REQ-0073 When transferring information between different security domains, decompose information into [Assignment: organization-defined policy-relevant subcomponents] for submission to policy enforcement mechanisms.
    - ITEM-0044 AC-4(14) Security or Privacy Policy Filter Constraints
      - REQ-0074 When transferring information between different security domains, implement [Assignment: organization-defined security or privacy policy filters] requiring fully enumerated formats that restrict data structure and content.
    - ITEM-0045 AC-4(15) Detection of Unsanctioned Information
      Related controls: [ITEM-0913].
      - REQ-0075 When transferring information between different security domains, examine the information for the presence of [Assignment: organization-defined unsanctioned information] and prohibit the transfer of such information in accordance with the [Assignment: organization-defined security or privacy policy].
    - ITEM-0046 AC-4(17) Domain Authentication
      Related controls: [ITEM-0341], [ITEM-0350], [ITEM-0385].
      - REQ-0076 Uniquely identify and authenticate source and destination points by [Selection (one or more): organization, system, application, service, individual] for information transfer.
    - ITEM-0047 AC-4(19) Validation of Metadata
      - REQ-0077 When transferring information between different security domains, implement [Assignment: organization-defined security or privacy policy filters] on metadata.
    - ITEM-0048 AC-4(20) Approved Solutions
      - REQ-0078 Employ [Assignment: organization-defined solutions in approved configurations] to control the flow of [Assignment: organization-defined information] across security domains.
    - ITEM-0049 AC-4(21) Physical or Logical Separation of Information Flows
      Related controls: [ITEM-0866].
      - REQ-0079 Separate information flows logically or physically using [Assignment: organization-defined mechanisms and/or techniques] to accomplish [Assignment: organization-defined required separations].
    - ITEM-0050 AC-4(22) Access Only
      - REQ-0080 Provide access from a single device to computing platforms, applications, or data residing in multiple different security domains, while preventing information flow between the different security domains.
    - ITEM-0051 AC-4(23) Modify Non-releasable Information
      - REQ-0081 When transferring information between different security domains, modify non-releasable information by implementing [Assignment: organization-defined modification action].
    - ITEM-0052 AC-4(24) Internal Normalized Format
      - REQ-0082 When transferring information between different security domains, parse incoming data into an internal normalized format and regenerate the data to be consistent with its intended specification.
    - ITEM-0053 AC-4(25) Data Sanitization
      Related controls: [ITEM-0477].
      - REQ-0083 When transferring information between different security domains, sanitize data to minimize [Selection (one or more): delivery of malicious content, command and control of malicious code, malicious code augmentation, and steganography-encoded data; spillage of sensitive information] in accordance with [Assignment: organization-defined policy].
    - ITEM-0054 AC-4(26) Audit Filtering Actions
      Related controls: [ITEM-0151], [ITEM-0152], [ITEM-0190].
      - REQ-0084 When transferring information between different security domains, record and audit content filtering actions and results for the information being filtered.
    - ITEM-0055 AC-4(27) Redundant/Independent Filtering Mechanisms
      - REQ-0085 When transferring information between different security domains, implement content filtering solutions that provide redundant and independent filtering mechanisms for each data type.
    - ITEM-0056 AC-4(28) Linear Filter Pipelines
      - REQ-0086 When transferring information between different security domains, implement a linear content filter pipeline that is enforced with discretionary and mandatory access controls.
    - ITEM-0057 AC-4(29) Filter Orchestration Engines
      - REQ-0087 (a) Content filtering mechanisms successfully complete execution without errors; and
      - REQ-0088 (b) Content filtering actions occur in the correct order and comply with [Assignment: organization-defined policy].
    - ITEM-0058 AC-4(30) Filter Mechanisms Using Multiple Processes
      - REQ-0089 When transferring information between different security domains, implement content filtering mechanisms using multiple processes.
    - ITEM-0059 AC-4(31) Failed Content Transfer Prevention
      - REQ-0090 When transferring information between different security domains, prevent the transfer of failed content to the receiving domain.
    - ITEM-0060 AC-4(32) Process Requirements for Information Transfer
      - REQ-0091 (a) Does not filter message content;
      - REQ-0092 (b) Validates filtering metadata;
      - REQ-0093 (c) Ensures the content associated with the filtering metadata has successfully completed filtering; and
      - REQ-0094 (d) Transfers the content to the destination filter pipeline.
  - ITEM-0061 AC-5 Separation of Duties
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0062], [ITEM-0175], [ITEM-0251], [ITEM-0282], [ITEM-0324], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0388], [ITEM-0444], [ITEM-0458], [ITEM-0594], [ITEM-0675], [ITEM-0749].
    - REQ-0095 (a) Identify and document [Assignment: organization-defined duties of individuals] ; and #moderate #high
    - REQ-0096 (b) Define system access authorizations to support separation of duties. #moderate #high
  - ITEM-0062 AC-6 Least Privilege
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0061], [ITEM-0091], [ITEM-0251], [ITEM-0282], [ITEM-0544], [ITEM-0568], [ITEM-0675], [ITEM-0736], [ITEM-0749], [ITEM-0877].
    - REQ-0097 Employ the principle of least privilege, allowing only authorized accesses for users (or processes acting on behalf of users) that are necessary to accomplish assigned organizational tasks. #moderate #high
    - ITEM-0063 AC-6(1) Authorize Access to Security Functions
      Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0175], [ITEM-0492].
      - REQ-0098 (a) [Assignment: organization-defined security functions (deployed in hardware, software, and firmware)] ; and #moderate #high
      - REQ-0099 (b) [Assignment: organization-defined security-relevant information]. #moderate #high
    - ITEM-0064 AC-6(2) Non-privileged Access for Nonsecurity Functions
      Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0545].
      - REQ-0100 Require that users of system accounts (or roles) with access to [Assignment: organization-defined security functions or security-relevant information] use non-privileged accounts or roles, when accessing nonsecurity functions. #moderate #high
    - ITEM-0065 AC-6(3) Network Access to Privileged Commands
      Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0115].
      - REQ-0101 Authorize network access to [Assignment: organization-defined privileged commands] only for [Assignment: organization-defined compelling operational needs] and document the rationale for such access in the security plan for the system. #high
    - ITEM-0066 AC-6(4) Separate Processing Domains
      Related controls: [ITEM-0030], [ITEM-0766], [ITEM-0769], [ITEM-0857], [ITEM-0866], [ITEM-0878].
      - REQ-0102 Provide separate processing domains to enable finer-grained allocation of user privileges.
    - ITEM-0067 AC-6(5) Privileged Accounts
      Related controls: [ITEM-0341], [ITEM-0444], [ITEM-0451].
      - REQ-0103 Restrict privileged accounts on the system to [Assignment: organization-defined personnel or roles]. #moderate #high
    - ITEM-0068 AC-6(6) Privileged Access by Non-organizational Users
      Related controls: [ITEM-0110], [ITEM-0115], [ITEM-0341], [ITEM-0379].
      - REQ-0104 Prohibit privileged access to the system by non-organizational users.
    - ITEM-0069 AC-6(7) Review of User Privileges
      Related controls: [ITEM-0220].
      - REQ-0105 (a) Review [Assignment: organization-defined frequency] the privileges assigned to [Assignment: organization-defined roles and classes] to validate the need for such privileges; and #moderate #high
      - REQ-0106 (b) Reassign or remove privileges, if necessary, to correctly reflect organizational mission and business needs. #moderate #high
    - ITEM-0070 AC-6(8) Privilege Levels for Code Execution
      - REQ-0107 Prevent the following software from executing at higher privilege levels than users executing the software: [Assignment: organization-defined software].
    - ITEM-0071 AC-6(9) Log Use of Privileged Functions
      Related controls: [ITEM-0151], [ITEM-0152], [ITEM-0190].
      - REQ-0108 Log the execution of privileged functions. #moderate #high
    - ITEM-0072 AC-6(10) Prohibit Non-privileged Users from Executing Privileged Functions
      - REQ-0109 Prevent non-privileged users from executing privileged functions. #moderate #high
  - ITEM-0073 AC-7 Unsuccessful Logon Attempts
    Related controls: [ITEM-0003], [ITEM-0078], [ITEM-0151], [ITEM-0163], [ITEM-0361].
    - REQ-0110 (a) Enforce a limit of [Assignment: organization-defined number] consecutive invalid logon attempts by a user during a [Assignment: organization-defined time period] ; and #low #moderate #high
    - REQ-0111 (b) Automatically [Selection (one or more): lock the account or node for [Assignment: organization-defined time period] ; lock the account or node until released by an administrator; delay next logon prompt per [Assignment: organization-defined delay algorithm] ; notify system administrator; take other [Assignment: organization-defined action] ] when the maximum number of unsuccessful attempts is exceeded. #low #moderate #high
    - ITEM-0074 AC-7(2) Purge or Wipe Mobile Device
      Related controls: [ITEM-0115], [ITEM-0475], [ITEM-0477].
      - REQ-0112 Purge or wipe information from [Assignment: organization-defined mobile devices] based on [Assignment: organization-defined purging or wiping requirements and techniques] after [Assignment: organization-defined number] consecutive, unsuccessful device logon attempts.
    - ITEM-0075 AC-7(3) Biometric Attempt Limiting
      Related controls: [ITEM-0350].
      - REQ-0113 Limit the number of unsuccessful biometric logon attempts to [Assignment: organization-defined number].
    - ITEM-0076 AC-7(4) Use of Alternate Authentication Factor
      Related controls: [ITEM-0350].
      - REQ-0114 (a) Allow the use of [Assignment: organization-defined authentication factors] that are different from the primary authentication factors after the number of organization-defined consecutive invalid logon attempts have been exceeded; and
      - REQ-0115 (b) Enforce a limit of [Assignment: organization-defined number] consecutive invalid logon attempts through use of the alternative factors by a user during a [Assignment: organization-defined time period].
  - ITEM-0077 AC-8 System Use Notification
    Related controls: [ITEM-0090], [ITEM-0545], [ITEM-0918].
    - REQ-0116 (a) Display [Assignment: organization-defined system use notification] to users before granting access to the system that provides privacy and security notices consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines and state that: #low #moderate #high
      (a)(1) Users are accessing a U.S. Government system;
      (a)(2) System usage may be monitored, recorded, and subject to audit;
      (a)(3) Unauthorized use of the system is prohibited and subject to criminal and civil penalties; and
      (a)(4) Use of the system indicates consent to monitoring and recording;
    - REQ-0117 (b) Retain the notification message or banner on the screen until users acknowledge the usage conditions and take explicit actions to log on to or further access the system; and #low #moderate #high
    - REQ-0118 (c) For publicly accessible systems: #low #moderate #high
      (c)(1) Display system use information [Assignment: organization-defined conditions] , before granting further access to the publicly accessible system;
      (c)(2) Display references, if any, to monitoring, recording, or auditing that are consistent with privacy accommodations for such systems that generally prohibit those activities; and
      (c)(3) Include a description of the authorized uses of the system.
  - ITEM-0078 AC-9 Previous Logon Notification
    Related controls: [ITEM-0073], [ITEM-0545].
    - REQ-0119 Notify the user, upon successful logon to the system, of the date and time of the last logon.
    - ITEM-0079 AC-9(1) Unsuccessful Logons
      - REQ-0120 Notify the user, upon successful logon, of the number of unsuccessful logon attempts since the last successful logon.
    - ITEM-0080 AC-9(2) Successful and Unsuccessful Logons
      - REQ-0121 Notify the user, upon successful logon, of the number of [Selection: successful logons; unsuccessful logon attempts; both] during [Assignment: organization-defined time period].
    - ITEM-0081 AC-9(3) Notification of Account Changes
      - REQ-0122 Notify the user, upon successful logon, of changes to [Assignment: organization-defined security-related characteristics or parameters] during [Assignment: organization-defined time period].
    - ITEM-0082 AC-9(4) Additional Logon Information
      - REQ-0123 Notify the user, upon successful logon, of the following additional information: [Assignment: organization-defined additional information].
  - ITEM-0083 AC-10 Concurrent Session Control
    Related controls: [ITEM-0843].
    - REQ-0124 Limit the number of concurrent sessions for each [Assignment: organization-defined account and/or account types] to [Assignment: organization-defined number]. #high
  - ITEM-0084 AC-11 Device Lock
    Related controls: [ITEM-0003], [ITEM-0073], [ITEM-0387], [ITEM-0545].
    - REQ-0125 (a) Prevent further access to the system by [Selection (one or more): initiating a device lock after [Assignment: organization-defined time period] of inactivity; requiring the user to initiate a device lock before leaving the system unattended] ; and #moderate #high
    - REQ-0126 (b) Retain the device lock until the user reestablishes access using established identification and authentication procedures. #moderate #high
    - ITEM-0085 AC-11(1) Pattern-hiding Displays
      - REQ-0127 Conceal, via the device lock, information previously visible on the display with a publicly viewable image. #moderate #high
  - ITEM-0086 AC-12 Session Termination
    Related controls: [ITEM-0451], [ITEM-0815], [ITEM-0843].
    - REQ-0128 Automatically terminate a user session after [Assignment: organization-defined conditions or trigger events]. #moderate #high
    - ITEM-0087 AC-12(1) User-initiated Logouts
      - REQ-0129 Provide a logout capability for user-initiated communications sessions whenever authentication is used to gain access to [Assignment: organization-defined information resources].
    - ITEM-0088 AC-12(2) Termination Message
      - REQ-0130 Display an explicit logout message to users indicating the termination of authenticated communications sessions.
    - ITEM-0089 AC-12(3) Timeout Warning Message
      - REQ-0131 Display an explicit message to users indicating that the session will end in [Assignment: organization-defined time].
  - ITEM-0090 AC-14 Permitted Actions Without Identification or Authentication
    Related controls: [ITEM-0077], [ITEM-0341], [ITEM-0544].
    - REQ-0132 (a) Identify [Assignment: organization-defined user actions] that can be performed on the system without identification or authentication consistent with organizational mission and business functions; and #low #moderate #high
    - REQ-0133 (b) Document and provide supporting rationale in the security plan for the system, user actions not requiring identification or authentication. #low #moderate #high
  - ITEM-0091 AC-16 Security and Privacy Attributes
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0062], [ITEM-0124], [ITEM-0132], [ITEM-0151], [ITEM-0183], [ITEM-0472], [ITEM-0540], [ITEM-0612], [ITEM-0615], [ITEM-0618], [ITEM-0816], [ITEM-0828], [ITEM-0972], [ITEM-0988].
    - REQ-0134 (a) Provide the means to associate [Assignment: organization-defined types of security and privacy attributes] with [Assignment: organization-defined security and privacy attribute values] for information in storage, in process, and/or in transmission;
    - REQ-0135 (b) Ensure that the attribute associations are made and retained with the information;
    - REQ-0136 (c) Establish the following permitted security and privacy attributes from the attributes defined in [AC-16a](#ac-16_smt.a) for [Assignment: organization-defined systems]: [Assignment: organization-defined security and privacy attributes];
    - REQ-0137 (d) Determine the following permitted attribute values or ranges for each of the established attributes: [Assignment: organization-defined attribute values or ranges];
    - REQ-0138 (e) Audit changes to attributes; and
    - REQ-0139 (f) Review [Assignment: organization-defined security and privacy attributes] for applicability [Assignment: organization-defined frequency].
    - ITEM-0092 AC-16(1) Dynamic Attribute Association
      - REQ-0140 Dynamically associate security and privacy attributes with [Assignment: organization-defined subjects and objects] in accordance with the following security and privacy policies as information is created and combined: [Assignment: organization-defined security and privacy policies].
    - ITEM-0093 AC-16(2) Attribute Value Changes by Authorized Individuals
      - REQ-0141 Provide authorized individuals (or processes acting on behalf of individuals) the capability to define or change the value of associated security and privacy attributes.
    - ITEM-0094 AC-16(3) Maintenance of Attribute Associations by System
      - REQ-0142 Maintain the association and integrity of [Assignment: organization-defined security and privacy attributes] to [Assignment: organization-defined subjects and objects].
    - ITEM-0095 AC-16(4) Association of Attributes by Authorized Individuals
      - REQ-0143 Provide the capability to associate [Assignment: organization-defined security and privacy attributes] with [Assignment: organization-defined subjects and objects] by authorized individuals (or processes acting on behalf of individuals).
    - ITEM-0096 AC-16(5) Attribute Displays on Objects to Be Output
      - REQ-0144 Display security and privacy attributes in human-readable form on each object that the system transmits to output devices to identify [Assignment: organization-defined instructions] using [Assignment: organization-defined naming conventions].
    - ITEM-0097 AC-16(6) Maintenance of Attribute Association
      - REQ-0145 Require personnel to associate and maintain the association of [Assignment: organization-defined security and privacy attributes] with [Assignment: organization-defined subjects and objects] in accordance with [Assignment: organization-defined security and privacy policies].
    - ITEM-0098 AC-16(7) Consistent Attribute Interpretation
      - REQ-0146 Provide a consistent interpretation of security and privacy attributes transmitted between distributed system components.
    - ITEM-0099 AC-16(8) Association Techniques and Technologies
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-0147 Implement [Assignment: organization-defined techniques and technologies] in associating security and privacy attributes to information.
    - ITEM-0100 AC-16(9) Attribute Reassignment — Regrading Mechanisms
      - REQ-0148 Change security and privacy attributes associated with information only via regrading mechanisms validated using [Assignment: organization-defined techniques or procedures].
    - ITEM-0101 AC-16(10) Attribute Configuration by Authorized Individuals
      - REQ-0149 Provide authorized individuals the capability to define or change the type and value of security and privacy attributes available for association with subjects and objects.
  - ITEM-0102 AC-17 Remote Access
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0030], [ITEM-0110], [ITEM-0115], [ITEM-0118], [ITEM-0212], [ITEM-0280], [ITEM-0341], [ITEM-0350], [ITEM-0379], [ITEM-0451], [ITEM-0534], [ITEM-0544], [ITEM-0545], [ITEM-0815], [ITEM-0818], [ITEM-0823], [ITEM-0918].
    - REQ-0150 (a) Establish and document usage restrictions, configuration/connection requirements, and implementation guidance for each type of remote access allowed; and #low #moderate #high
    - REQ-0151 (b) Authorize each type of remote access to the system prior to allowing such connections. #low #moderate #high
    - ITEM-0103 AC-17(1) Monitoring and Control
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0190], [ITEM-0199].
      - REQ-0152 Employ automated mechanisms to monitor and control remote access methods. #moderate #high
    - ITEM-0104 AC-17(2) Protection of Confidentiality and Integrity Using Encryption
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-0153 Implement cryptographic mechanisms to protect the confidentiality and integrity of remote access sessions. #moderate #high
    - ITEM-0105 AC-17(3) Managed Access Control Points
      Related controls: [ITEM-0782].
      - REQ-0154 Route remote accesses through authorized and managed network access control points. #moderate #high
    - ITEM-0106 AC-17(4) Privileged Commands and Access
      Related controls: [ITEM-0062], [ITEM-0818], [ITEM-0823].
      - REQ-0155 (a) Authorize the execution of privileged commands and access to security-relevant information via remote access only in a format that provides assessable evidence and for the following needs: [Assignment: organization-defined needs] ; and #moderate #high
      - REQ-0156 (b) Document the rationale for remote access in the security plan for the system. #moderate #high
    - ITEM-0107 AC-17(6) Protection of Mechanism Information
      Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0604].
      - REQ-0157 Protect information about remote access mechanisms from unauthorized use and disclosure.
    - ITEM-0108 AC-17(9) Disconnect or Disable Access
      - REQ-0158 Provide the capability to disconnect or disable remote access to the system within [Assignment: organization-defined time period].
    - ITEM-0109 AC-17(10) Authenticate Remote Commands
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0843].
      - REQ-0159 Implement [Assignment: organization-defined mechanisms] to authenticate [Assignment: organization-defined remote commands].
  - ITEM-0110 AC-18 Wireless Access
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0102], [ITEM-0115], [ITEM-0230], [ITEM-0259], [ITEM-0341], [ITEM-0350], [ITEM-0379], [ITEM-0545], [ITEM-0881], [ITEM-0892], [ITEM-0918].
    - REQ-0160 (a) Establish configuration requirements, connection requirements, and implementation guidance for each type of wireless access; and #low #moderate #high
    - REQ-0161 (b) Authorize each type of wireless access to the system prior to allowing such connections. #low #moderate #high
    - ITEM-0111 AC-18(1) Authentication and Encryption
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-0162 Protect wireless access to the system using authentication of [Selection (one or more): users; devices] and encryption. #moderate #high
    - ITEM-0112 AC-18(3) Disable Wireless Networking
      - REQ-0163 Disable, when not intended for use, wireless networking capabilities embedded within system components prior to issuance and deployment. #moderate #high
    - ITEM-0113 AC-18(4) Restrict Configurations by Users
      Related controls: [ITEM-0782], [ITEM-0824].
      - REQ-0164 Identify and explicitly authorize users allowed to independently configure wireless networking capabilities. #high
    - ITEM-0114 AC-18(5) Antennas and Transmission Power Levels
      Related controls: [ITEM-0536].
      - REQ-0165 Select radio antennas and calibrate transmission power levels to reduce the probability that signals from wireless access points can be received outside of organization-controlled boundaries. #high
  - ITEM-0115 AC-19 Access Control for Mobile Devices
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0073], [ITEM-0084], [ITEM-0102], [ITEM-0110], [ITEM-0118], [ITEM-0230], [ITEM-0234], [ITEM-0256], [ITEM-0341], [ITEM-0350], [ITEM-0471], [ITEM-0473], [ITEM-0475], [ITEM-0483], [ITEM-0545], [ITEM-0782], [ITEM-0868], [ITEM-0892], [ITEM-0913], [ITEM-0918].
    - REQ-0166 (a) Establish configuration requirements, connection requirements, and implementation guidance for organization-controlled mobile devices, to include when such devices are outside of controlled areas; and #low #moderate #high
    - REQ-0167 (b) Authorize the connection of mobile devices to organizational systems. #low #moderate #high
    - ITEM-0116 AC-19(4) Restrictions for Classified Information
      Related controls: [ITEM-0269], [ITEM-0409].
      - REQ-0168 (a) Prohibit the use of unclassified mobile devices in facilities containing systems processing, storing, or transmitting classified information unless specifically permitted by the authorizing official; and
      - REQ-0169 (b) Enforce the following restrictions on individuals permitted by the authorizing official to use unclassified mobile devices in facilities containing systems processing, storing, or transmitting classified information:
        (b)(1) Connection of unclassified mobile devices to classified systems is prohibited;
        (b)(2) Connection of unclassified mobile devices to unclassified systems requires approval from the authorizing official;
        (b)(3) Use of internal or external modems or wireless interfaces within the unclassified mobile devices is prohibited; and
        (b)(4) Unclassified mobile devices and the information stored on those devices are subject to random reviews and inspections by [Assignment: organization-defined security officials] , and if classified information is found, the incident handling policy is followed.
      - REQ-0170 (c) Restrict the connection of classified mobile devices to classified systems in accordance with [Assignment: organization-defined security policies].
    - ITEM-0117 AC-19(5) Full Device or Container-based Encryption
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0851].
      - REQ-0171 Employ [Selection: full-device encryption; container-based encryption] to protect the confidentiality and integrity of information on [Assignment: organization-defined mobile devices]. #moderate #high
  - ITEM-0118 AC-20 Use of External Systems
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0102], [ITEM-0115], [ITEM-0212], [ITEM-0544], [ITEM-0545], [ITEM-0709], [ITEM-0782].
    - REQ-0172 (a) [Selection (one or more): establish [Assignment: organization-defined terms and conditions] ; identify [Assignment: organization-defined controls asserted] ] , consistent with the trust relationships established with other organizations owning, operating, and/or maintaining external systems, allowing authorized individuals to: #low #moderate #high
      (a)(1) Access the system from external systems; and
      (a)(2) Process, store, or transmit organization-controlled information using external systems; or
    - REQ-0173 (b) Prohibit the use of [Assignment: organization-defined prohibited types of external systems]. #low #moderate #high
    - ITEM-0119 AC-20(1) Limits on Authorized Use
      Related controls: [ITEM-0208].
      - REQ-0174 (a) Verification of the implementation of controls on the external system as specified in the organization’s security and privacy policies and security and privacy plans; or #moderate #high
      - REQ-0175 (b) Retention of approved system connection or processing agreements with the organizational entity hosting the external system. #moderate #high
    - ITEM-0120 AC-20(2) Portable Storage Devices — Restricted Use
      Related controls: [ITEM-0483], [ITEM-0886].
      - REQ-0176 Restrict the use of organization-controlled portable storage devices by authorized individuals on external systems using [Assignment: organization-defined restrictions]. #moderate #high
    - ITEM-0121 AC-20(3) Non-organizationally Owned Systems — Restricted Use
      - REQ-0177 Restrict the use of non-organizationally owned systems or system components to process, store, or transmit organizational information using [Assignment: organization-defined restrictions].
    - ITEM-0122 AC-20(4) Network Accessible Storage Devices — Prohibited Use
      - REQ-0178 Prohibit the use of [Assignment: organization-defined network-accessible storage devices] in external systems.
    - ITEM-0123 AC-20(5) Portable Storage Devices — Prohibited Use
      Related controls: [ITEM-0483], [ITEM-0545], [ITEM-0604], [ITEM-0886].
      - REQ-0179 Prohibit the use of organization-controlled portable storage devices by authorized individuals on external systems.
  - ITEM-0124 AC-21 Information Sharing
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0091], [ITEM-0612], [ITEM-0628], [ITEM-0636], [ITEM-0824].
    - REQ-0180 (a) Enable authorized users to determine whether access authorizations assigned to a sharing partner match the information’s access and use restrictions for [Assignment: organization-defined information-sharing circumstances] ; and #moderate #high
    - REQ-0181 (b) Employ [Assignment: organization-defined automated mechanisms] to assist users in making information sharing and collaboration decisions. #moderate #high
    - ITEM-0125 AC-21(1) Automated Decision Support
      - REQ-0182 Employ [Assignment: organization-defined automated mechanisms] to enforce information-sharing decisions by authorized users based on access authorizations of sharing partners and access restrictions on information to be shared.
    - ITEM-0126 AC-21(2) Information Search and Retrieval
      - REQ-0183 Implement information search and retrieval services that enforce [Assignment: organization-defined information-sharing restrictions].
  - ITEM-0127 AC-22 Publicly Accessible Content
    Related controls: [ITEM-0016], [ITEM-0135], [ITEM-0142], [ITEM-0195].
    - REQ-0184 (a) Designate individuals authorized to make information publicly accessible; #low #moderate #high
    - REQ-0185 (b) Train authorized individuals to ensure that publicly accessible information does not contain nonpublic information; #low #moderate #high
    - REQ-0186 (c) Review the proposed content of information prior to posting onto the publicly accessible system to ensure that nonpublic information is not included; and #low #moderate #high
    - REQ-0187 (d) Review the content on the publicly accessible system for nonpublic information [Assignment: organization-defined frequency] and remove such information, if discovered. #low #moderate #high
  - ITEM-0128 AC-23 Data Mining Protection
    Related controls: [ITEM-0568], [ITEM-0612].
    - REQ-0188 Employ [Assignment: organization-defined techniques] for [Assignment: organization-defined data storage objects] to detect and protect against unauthorized data mining.
  - ITEM-0129 AC-24 Access Control Decisions
    Related controls: [ITEM-0003], [ITEM-0016].
    - REQ-0189 [Selection (one or more): establish procedures; implement mechanisms] to ensure [Assignment: organization-defined access control decisions] are applied to each access request prior to access enforcement.
    - ITEM-0130 AC-24(1) Transmit Access Authorization Information
      Related controls: [ITEM-0183].
      - REQ-0190 Transmit [Assignment: organization-defined access authorization information] using [Assignment: organization-defined controls] to [Assignment: organization-defined systems] that enforce access control decisions.
    - ITEM-0131 AC-24(2) No User or Process Identity
      - REQ-0191 Enforce access control decisions based on [Assignment: organization-defined security or privacy attributes] that do not include the identity of the user or process acting on behalf of the user.
  - ITEM-0132 AC-25 Reference Monitor
    Related controls: [ITEM-0016], [ITEM-0091], [ITEM-0675], [ITEM-0749], [ITEM-0769], [ITEM-0816], [ITEM-0878], [ITEM-0976].
    - REQ-0192 Implement a reference monitor for [Assignment: organization-defined access control policies] that is tamperproof, always invoked, and small enough to be subject to analysis and testing, the completeness of which can be assured.
- ITEM-0133 Awareness and Training
  The awareness and training family (AT) of NIST SP 800-53 Revision 5.
  - ITEM-0134 AT-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0193 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] awareness and training policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the awareness and training policy and the associated awareness and training controls;
    - REQ-0194 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the awareness and training policy and procedures; and #low #moderate #high #privacy
    - REQ-0195 (c) Review and update the current awareness and training: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0135 AT-2 Literacy Training and Awareness
    Related controls: [ITEM-0016], [ITEM-0102], [ITEM-0127], [ITEM-0142], [ITEM-0147], [ITEM-0299], [ITEM-0354], [ITEM-0401], [ITEM-0431], [ITEM-0436], [ITEM-0545], [ITEM-0569], [ITEM-0579], [ITEM-0607], [ITEM-0612], [ITEM-0675], [ITEM-0748].
    - REQ-0196 (a) Provide security and privacy literacy training to system users (including managers, senior executives, and contractors): #low #moderate #high #privacy
      (a)(1) As part of initial training for new users and [Assignment: organization-defined frequency] thereafter; and
      (a)(2) When required by system changes or following [Assignment: organization-defined events];
    - REQ-0197 (b) Employ the following techniques to increase the security and privacy awareness of system users [Assignment: organization-defined awareness techniques]; #low #moderate #high #privacy
    - REQ-0198 (c) Update literacy training and awareness content [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and #low #moderate #high #privacy
    - REQ-0199 (d) Incorporate lessons learned from internal or external security incidents or breaches into literacy training and awareness techniques. #low #moderate #high #privacy
    - ITEM-0136 AT-2(1) Practical Exercises
      Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0302], [ITEM-0405].
      - REQ-0200 Provide practical exercises in literacy training that simulate events and incidents.
    - ITEM-0137 AT-2(2) Insider Threat
      Related controls: [ITEM-0568].
      - REQ-0201 Provide literacy training on recognizing and reporting potential indicators of insider threat. #low #moderate #high
    - ITEM-0138 AT-2(3) Social Engineering and Mining
      - REQ-0202 Provide literacy training on recognizing and reporting potential and actual instances of social engineering and social mining. #moderate #high
    - ITEM-0139 AT-2(4) Suspicious Communications and Anomalous System Behavior
      - REQ-0203 Provide literacy training on recognizing suspicious communications and anomalous behavior in organizational systems using [Assignment: organization-defined indicators of malicious code].
    - ITEM-0140 AT-2(5) Advanced Persistent Threat
      - REQ-0204 Provide literacy training on the advanced persistent threat.
    - ITEM-0141 AT-2(6) Cyber Threat Environment
      Related controls: [ITEM-0636].
      - REQ-0205 (a) Provide literacy training on the cyber threat environment; and
      - REQ-0206 (b) Reflect current cyber threat information in system operations.
  - ITEM-0142 AT-3 Role-based Training
    Related controls: [ITEM-0016], [ITEM-0102], [ITEM-0127], [ITEM-0135], [ITEM-0147], [ITEM-0299], [ITEM-0401], [ITEM-0409], [ITEM-0431], [ITEM-0436], [ITEM-0545], [ITEM-0569], [ITEM-0581], [ITEM-0607], [ITEM-0609], [ITEM-0658], [ITEM-0675], [ITEM-0726], [ITEM-0748], [ITEM-1020], [ITEM-1023], [ITEM-1030].
    - REQ-0207 (a) Provide role-based security and privacy training to personnel with the following roles and responsibilities: [Assignment: organization-defined roles and responsibilities]: #low #moderate #high #privacy
      (a)(1) Before authorizing access to the system, information, or performing assigned duties, and [Assignment: organization-defined frequency] thereafter; and
      (a)(2) When required by system changes;
    - REQ-0208 (b) Update role-based training content [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and #low #moderate #high #privacy
    - REQ-0209 (c) Incorporate lessons learned from internal or external security incidents or breaches into role-based training. #low #moderate #high #privacy
    - ITEM-0143 AT-3(1) Environmental Controls
      Related controls: [ITEM-0491], [ITEM-0519], [ITEM-0524], [ITEM-0528], [ITEM-0531].
      - REQ-0210 Provide [Assignment: organization-defined personnel or roles] with initial and [Assignment: organization-defined frequency] training in the employment and operation of environmental controls.
    - ITEM-0144 AT-3(2) Physical Security Controls
      Related controls: [ITEM-0492], [ITEM-0496], [ITEM-0504].
      - REQ-0211 Provide [Assignment: organization-defined personnel or roles] with initial and [Assignment: organization-defined frequency] training in the employment and operation of physical security controls.
    - ITEM-0145 AT-3(3) Practical Exercises
      - REQ-0212 Provide practical exercises in security and privacy training that reinforce training objectives.
    - ITEM-0146 AT-3(5) Processing Personally Identifiable Information
      Related controls: [ITEM-0612], [ITEM-0615], [ITEM-0622], [ITEM-0625].
      - REQ-0213 Provide [Assignment: organization-defined personnel or roles] with initial and [Assignment: organization-defined frequency] training in the employment and operation of personally identifiable information processing and transparency controls. #privacy
  - ITEM-0147 AT-4 Training Records
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0299], [ITEM-0401], [ITEM-0570], [ITEM-0972].
    - REQ-0214 (a) Document and monitor information security and privacy training activities, including security and privacy awareness training and specific role-based security and privacy training; and #low #moderate #high #privacy
    - REQ-0215 (b) Retain individual training records for [Assignment: organization-defined time period]. #low #moderate #high #privacy
  - ITEM-0148 AT-6 Training Feedback
    - REQ-0216 Provide feedback on organizational training results to the following personnel [Assignment: organization-defined frequency]: [Assignment: organization-defined personnel].
- ITEM-0149 Audit and Accountability
  The audit and accountability family (AU) of NIST SP 800-53 Revision 5.
  - ITEM-0150 AU-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0217 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] audit and accountability policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the audit and accountability policy and the associated audit and accountability controls;
    - REQ-0218 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the audit and accountability policy and procedures; and #low #moderate #high #privacy
    - REQ-0219 (c) Review and update the current audit and accountability: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0151 AU-2 Event Logging
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0062], [ITEM-0073], [ITEM-0077], [ITEM-0091], [ITEM-0102], [ITEM-0152], [ITEM-0155], [ITEM-0157], [ITEM-0163], [ITEM-0172], [ITEM-0188], [ITEM-0190], [ITEM-0239], [ITEM-0251], [ITEM-0256], [ITEM-0287], [ITEM-0350], [ITEM-0451], [ITEM-0473], [ITEM-0496], [ITEM-0579], [ITEM-0612], [ITEM-0628], [ITEM-0652], [ITEM-0675], [ITEM-0782], [ITEM-0833], [ITEM-0913], [ITEM-0918], [ITEM-0947], [ITEM-0964], [ITEM-0971].
    - REQ-0220 (a) Identify the types of events that the system is capable of logging in support of the audit function: [Assignment: organization-defined event types]; #low #moderate #high #privacy
    - REQ-0221 (b) Coordinate the event logging function with other organizational entities requiring audit-related information to guide and inform the selection criteria for events to be logged; #low #moderate #high #privacy
    - REQ-0222 (c) Specify the following event types for logging within the system: [Assignment: organization-defined event types (subset of the event types defined in [AU-2a.](#au-2_smt.a)) along with the frequency of (or situation requiring) logging for each identified event type]; #low #moderate #high #privacy
    - REQ-0223 (d) Provide a rationale for why the event types selected for logging are deemed to be adequate to support after-the-fact investigations of incidents; and #low #moderate #high #privacy
    - REQ-0224 (e) Review and update the event types selected for logging [Assignment: organization-defined frequency]. #low #moderate #high #privacy
  - ITEM-0152 AU-3 Content of Audit Records
    Related controls: [ITEM-0151], [ITEM-0174], [ITEM-0190], [ITEM-0199], [ITEM-0451], [ITEM-0551], [ITEM-0675], [ITEM-0947], [ITEM-0971].
    - REQ-0225 (a) What type of event occurred; #low #moderate #high
    - REQ-0226 (b) When the event occurred; #low #moderate #high
    - REQ-0227 (c) Where the event occurred; #low #moderate #high
    - REQ-0228 (d) Source of the event; #low #moderate #high
    - REQ-0229 (e) Outcome of the event; and #low #moderate #high
    - REQ-0230 (f) Identity of any individuals, subjects, or objects/entities associated with the event. #low #moderate #high
    - ITEM-0153 AU-3(1) Additional Audit Information
      - REQ-0231 Generate audit records containing the following additional information: [Assignment: organization-defined additional information]. #moderate #high
    - ITEM-0154 AU-3(3) Limit Personally Identifiable Information Elements
      Related controls: [ITEM-0636].
      - REQ-0232 Limit personally identifiable information contained in audit records to the following elements identified in the privacy risk assessment: [Assignment: organization-defined elements]. #privacy
  - ITEM-0155 AU-4 Audit Log Storage Capacity
    Related controls: [ITEM-0151], [ITEM-0157], [ITEM-0163], [ITEM-0172], [ITEM-0175], [ITEM-0188], [ITEM-0190], [ITEM-0199], [ITEM-0918].
    - REQ-0233 Allocate audit log storage capacity to accommodate [Assignment: organization-defined audit log retention requirements]. #low #moderate #high
    - ITEM-0156 AU-4(1) Transfer to Alternate Storage
      - REQ-0234 Transfer audit logs [Assignment: organization-defined frequency] to a different system, system component, or media other than the system or system component conducting the logging.
  - ITEM-0157 AU-5 Response to Audit Logging Process Failures
    Related controls: [ITEM-0151], [ITEM-0155], [ITEM-0172], [ITEM-0175], [ITEM-0188], [ITEM-0190], [ITEM-0199], [ITEM-0918], [ITEM-0972].
    - REQ-0235 (a) Alert [Assignment: organization-defined personnel or roles] within [Assignment: organization-defined time period] in the event of an audit logging process failure; and #low #moderate #high
    - REQ-0236 (b) Take the following additional actions: [Assignment: organization-defined additional actions]. #low #moderate #high
    - ITEM-0158 AU-5(1) Storage Capacity Warning
      - REQ-0237 Provide a warning to [Assignment: organization-defined personnel, roles, and/or locations] within [Assignment: organization-defined time period] when allocated audit log storage volume reaches [Assignment: organization-defined percentage] of repository maximum audit log storage capacity. #high
    - ITEM-0159 AU-5(2) Real-time Alerts
      - REQ-0238 Provide an alert within [Assignment: organization-defined real-time period] to [Assignment: organization-defined personnel, roles, and/or locations] when the following audit failure events occur: [Assignment: organization-defined audit logging failure events requiring real-time alerts]. #high
    - ITEM-0160 AU-5(3) Configurable Traffic Volume Thresholds
      - REQ-0239 Enforce configurable network communications traffic volume thresholds reflecting limits on audit log storage capacity and [Selection (one or more): reject; delay] network traffic above those thresholds.
    - ITEM-0161 AU-5(4) Shutdown on Failure
      - REQ-0240 Invoke a [Selection (one or more): full system shutdown; partial system shutdown; degraded operational mode with limited mission or business functionality available] in the event of [Assignment: organization-defined audit logging failures] , unless an alternate audit logging capability exists.
    - ITEM-0162 AU-5(5) Alternate Audit Logging Capability
      Related controls: [ITEM-0175].
      - REQ-0241 Provide an alternate audit logging capability in the event of a failure in primary audit logging capability that implements [Assignment: organization-defined alternate audit logging functionality].
  - ITEM-0163 AU-6 Audit Record Review, Analysis, and Reporting
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0061], [ITEM-0062], [ITEM-0073], [ITEM-0102], [ITEM-0172], [ITEM-0202], [ITEM-0208], [ITEM-0220], [ITEM-0234], [ITEM-0251], [ITEM-0256], [ITEM-0280], [ITEM-0282], [ITEM-0341], [ITEM-0350], [ITEM-0361], [ITEM-0379], [ITEM-0425], [ITEM-0451], [ITEM-0473], [ITEM-0496], [ITEM-0507], [ITEM-0641], [ITEM-0675], [ITEM-0782], [ITEM-0913], [ITEM-0918], [ITEM-0947].
    - REQ-0242 (a) Review and analyze system audit records [Assignment: organization-defined frequency] for indications of [Assignment: organization-defined inappropriate or unusual activity] and the potential impact of the inappropriate or unusual activity; #low #moderate #high
    - REQ-0243 (b) Report findings to [Assignment: organization-defined personnel or roles] ; and #low #moderate #high
    - REQ-0244 (c) Adjust the level of audit record review, analysis, and reporting within the system when there is a change in risk based on law enforcement information, intelligence information, or other credible sources of information. #low #moderate #high
    - ITEM-0164 AU-6(1) Automated Process Integration
      Related controls: [ITEM-0562].
      - REQ-0245 Integrate audit record review, analysis, and reporting processes using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0165 AU-6(3) Correlate Audit Record Repositories
      Related controls: [ITEM-0190], [ITEM-0409].
      - REQ-0246 Analyze and correlate audit records across different repositories to gain organization-wide situational awareness. #moderate #high
    - ITEM-0166 AU-6(4) Central Review and Analysis
      Related controls: [ITEM-0151], [ITEM-0190].
      - REQ-0247 Provide and implement the capability to centrally review and analyze audit records from multiple components within the system.
    - ITEM-0167 AU-6(5) Integrated Analysis of Audit Records
      Related controls: [ITEM-0190], [ITEM-0409].
      - REQ-0248 Integrate analysis of audit records with analysis of [Selection (one or more): vulnerability scanning information; performance data; system monitoring information; [Assignment: organization-defined data/information collected from other sources] ] to further enhance the ability to identify inappropriate or unusual activity. #high
    - ITEM-0168 AU-6(6) Correlation with Physical Monitoring
      - REQ-0249 Correlate information from audit records with information obtained from monitoring physical access to further enhance the ability to identify suspicious, inappropriate, unusual, or malevolent activity. #high
    - ITEM-0169 AU-6(7) Permitted Actions
      - REQ-0250 Specify the permitted actions for each [Selection (one or more): system process; role; user] associated with the review, analysis, and reporting of audit record information.
    - ITEM-0170 AU-6(8) Full Text Analysis of Privileged Commands
      Related controls: [ITEM-0152], [ITEM-0175], [ITEM-0188], [ITEM-0190].
      - REQ-0251 Perform a full text analysis of logged privileged commands in a physically distinct component or subsystem of the system, or other system that is dedicated to that analysis.
    - ITEM-0171 AU-6(9) Correlation with Information from Nontechnical Sources
      Related controls: [ITEM-0568].
      - REQ-0252 Correlate information from nontechnical sources with audit record information to enhance organization-wide situational awareness.
  - ITEM-0172 AU-7 Audit Record Reduction and Report Generation
    Related controls: [ITEM-0003], [ITEM-0151], [ITEM-0152], [ITEM-0155], [ITEM-0157], [ITEM-0163], [ITEM-0190], [ITEM-0202], [ITEM-0251], [ITEM-0361], [ITEM-0409], [ITEM-0568], [ITEM-0918].
    - REQ-0253 (a) Supports on-demand audit record review, analysis, and reporting requirements and after-the-fact investigations of incidents; and #moderate #high
    - REQ-0254 (b) Does not alter the original content or time ordering of audit records. #moderate #high
    - ITEM-0173 AU-7(1) Automatic Processing
      - REQ-0255 Provide and implement the capability to process, sort, and search audit records for events of interest based on the following content: [Assignment: organization-defined fields within audit records]. #moderate #high
  - ITEM-0174 AU-8 Time Stamps
    Related controls: [ITEM-0152], [ITEM-0190], [ITEM-0199], [ITEM-0894].
    - REQ-0256 (a) Use internal system clocks to generate time stamps for audit records; and #low #moderate #high
    - REQ-0257 (b) Record time stamps for audit records that meet [Assignment: organization-defined granularity of time measurement] and that use Coordinated Universal Time, have a fixed local time offset from Coordinated Universal Time, or that include the local time offset as part of the time stamp. #low #moderate #high
  - ITEM-0175 AU-9 Protection of Audit Information
    Related controls: [ITEM-0016], [ITEM-0062], [ITEM-0163], [ITEM-0188], [ITEM-0199], [ITEM-0471], [ITEM-0473], [ITEM-0492], [ITEM-0496], [ITEM-0507], [ITEM-0675], [ITEM-0809], [ITEM-0918].
    - REQ-0258 (a) Protect audit information and audit logging tools from unauthorized access, modification, and deletion; and #low #moderate #high
    - REQ-0259 (b) Alert [Assignment: organization-defined personnel or roles] upon detection of unauthorized access, modification, or deletion of audit information. #low #moderate #high
    - ITEM-0176 AU-9(1) Hardware Write-once Media
      Related controls: [ITEM-0155], [ITEM-0157].
      - REQ-0260 Write audit trails to hardware-enforced, write-once media.
    - ITEM-0177 AU-9(2) Store on Separate Physical Systems or Components
      Related controls: [ITEM-0155], [ITEM-0157].
      - REQ-0261 Store audit records [Assignment: organization-defined frequency] in a repository that is part of a physically different system or system component than the system or component being audited. #high
    - ITEM-0178 AU-9(3) Cryptographic Protection
      Related controls: [ITEM-0183], [ITEM-0818], [ITEM-0823].
      - REQ-0262 Implement cryptographic mechanisms to protect the integrity of audit information and audit tools. #high
    - ITEM-0179 AU-9(4) Access by Subset of Privileged Users
      Related controls: [ITEM-0061].
      - REQ-0263 Authorize access to management of audit logging functionality to only [Assignment: organization-defined subset of privileged users or roles]. #moderate #high
    - ITEM-0180 AU-9(5) Dual Authorization
      Related controls: [ITEM-0016].
      - REQ-0264 Enforce dual authorization for [Selection (one or more): movement; deletion] of [Assignment: organization-defined audit information].
    - ITEM-0181 AU-9(6) Read-only Access
      - REQ-0265 Authorize read-only access to audit information to [Assignment: organization-defined subset of privileged users or roles].
    - ITEM-0182 AU-9(7) Store on Component with Different Operating System
      Related controls: [ITEM-0155], [ITEM-0157], [ITEM-0188], [ITEM-0855].
      - REQ-0266 Store audit information on a component running a different operating system than the system or component being audited.
  - ITEM-0183 AU-10 Non-repudiation
    Related controls: [ITEM-0175], [ITEM-0568], [ITEM-0675], [ITEM-0809], [ITEM-0818], [ITEM-0823], [ITEM-0828], [ITEM-0832], [ITEM-0843].
    - REQ-0267 Provide irrefutable evidence that an individual (or process acting on behalf of an individual) has performed [Assignment: organization-defined actions]. #high
    - ITEM-0184 AU-10(1) Association of Identities
      Related controls: [ITEM-0030], [ITEM-0091].
      - REQ-0268 (a) Bind the identity of the information producer with the information to [Assignment: organization-defined strength of binding] ; and
      - REQ-0269 (b) Provide the means for authorized individuals to determine the identity of the producer of the information.
    - ITEM-0185 AU-10(2) Validate Binding of Information Producer Identity
      Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0091].
      - REQ-0270 (a) Validate the binding of the information producer identity to the information at [Assignment: organization-defined frequency] ; and
      - REQ-0271 (b) Perform [Assignment: organization-defined actions] in the event of a validation error.
    - ITEM-0186 AU-10(3) Chain of Custody
      Related controls: [ITEM-0030], [ITEM-0091].
      - REQ-0272 Maintain reviewer or releaser credentials within the established chain of custody for information reviewed or released.
    - ITEM-0187 AU-10(4) Validate Binding of Information Reviewer Identity
      Related controls: [ITEM-0030], [ITEM-0091].
      - REQ-0273 (a) Validate the binding of the information reviewer identity to the information at the transfer or release points prior to release or transfer between [Assignment: organization-defined security domains] ; and
      - REQ-0274 (b) Perform [Assignment: organization-defined actions] in the event of a validation error.
  - ITEM-0188 AU-11 Audit Record Retention
    Related controls: [ITEM-0151], [ITEM-0155], [ITEM-0157], [ITEM-0163], [ITEM-0175], [ITEM-0199], [ITEM-0477], [ITEM-0641], [ITEM-0972].
    - REQ-0275 Retain audit records for [Assignment: organization-defined time period] to provide support for after-the-fact investigations of incidents and to meet regulatory and organizational information retention requirements. #low #moderate #high #privacy
    - ITEM-0189 AU-11(1) Long-term Retrieval Capability
      - REQ-0276 Employ [Assignment: organization-defined measures] to ensure that long-term audit records generated by the system can be retrieved.
  - ITEM-0190 AU-12 Audit Record Generation
    Related controls: [ITEM-0062], [ITEM-0102], [ITEM-0151], [ITEM-0152], [ITEM-0155], [ITEM-0157], [ITEM-0163], [ITEM-0172], [ITEM-0199], [ITEM-0251], [ITEM-0451], [ITEM-0473], [ITEM-0568], [ITEM-0675], [ITEM-0833], [ITEM-0913], [ITEM-0918], [ITEM-0947], [ITEM-0964].
    - REQ-0277 (a) Provide audit record generation capability for the event types the system is capable of auditing as defined in [AU-2a](#au-2_smt.a) on [Assignment: organization-defined system components]; #low #moderate #high
    - REQ-0278 (b) Allow [Assignment: organization-defined personnel or roles] to select the event types that are to be logged by specific components of the system; and #low #moderate #high
    - REQ-0279 (c) Generate audit records for the event types defined in [AU-2c](#au-2_smt.c) that include the audit record content defined in [AU-3](#au-3). #low #moderate #high
    - ITEM-0191 AU-12(1) System-wide and Time-correlated Audit Trail
      Related controls: [ITEM-0174], [ITEM-0894].
      - REQ-0280 Compile audit records from [Assignment: organization-defined system components] into a system-wide (logical or physical) audit trail that is time-correlated to within [Assignment: organization-defined level of tolerance]. #high
    - ITEM-0192 AU-12(2) Standardized Formats
      - REQ-0281 Produce a system-wide (logical or physical) audit trail composed of audit records in a standardized format.
    - ITEM-0193 AU-12(3) Changes by Authorized Individuals
      Related controls: [ITEM-0016].
      - REQ-0282 Provide and implement the capability for [Assignment: organization-defined individuals or roles] to change the logging to be performed on [Assignment: organization-defined system components] based on [Assignment: organization-defined selectable event criteria] within [Assignment: organization-defined time thresholds]. #high
    - ITEM-0194 AU-12(4) Query Parameter Audits of Personally Identifiable Information
      - REQ-0283 Provide and implement the capability for auditing the parameters of user query events for data sets containing personally identifiable information.
  - ITEM-0195 AU-13 Monitoring for Information Disclosure
    Related controls: [ITEM-0127], [ITEM-0496], [ITEM-0568], [ITEM-0641], [ITEM-0782], [ITEM-1003].
    - REQ-0284 (a) Monitor [Assignment: organization-defined open-source information and/or information sites] [Assignment: organization-defined frequency] for evidence of unauthorized disclosure of organizational information; and
    - REQ-0285 (b) If an information disclosure is discovered:
      (b)(1) Notify [Assignment: organization-defined personnel or roles] ; and
      (b)(2) Take the following additional actions: [Assignment: organization-defined additional actions].
    - ITEM-0196 AU-13(1) Use of Automated Tools
      - REQ-0286 Monitor open-source information and information sites using [Assignment: organization-defined automated mechanisms].
    - ITEM-0197 AU-13(2) Review of Monitored Sites
      - REQ-0287 Review the list of open-source information sites being monitored [Assignment: organization-defined frequency].
    - ITEM-0198 AU-13(3) Unauthorized Replication of Information
      - REQ-0288 Employ discovery techniques, processes, and tools to determine if external entities are replicating organizational information in an unauthorized manner.
  - ITEM-0199 AU-14 Session Audit
    Related controls: [ITEM-0016], [ITEM-0077], [ITEM-0151], [ITEM-0152], [ITEM-0155], [ITEM-0157], [ITEM-0174], [ITEM-0175], [ITEM-0188], [ITEM-0190].
    - REQ-0289 (a) Provide and implement the capability for [Assignment: organization-defined users or roles] to [Selection (one or more): record; view; hear; log] the content of a user session under [Assignment: organization-defined circumstances] ; and
    - REQ-0290 (b) Develop, integrate, and use session auditing activities in consultation with legal counsel and in accordance with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines.
    - ITEM-0200 AU-14(1) System Start-up
      - REQ-0291 Initiate session audits automatically at system start-up.
    - ITEM-0201 AU-14(3) Remote Viewing and Listening
      Related controls: [ITEM-0102].
      - REQ-0292 Provide and implement the capability for authorized users to remotely view and hear content related to an established user session in real time.
  - ITEM-0202 AU-16 Cross-organizational Audit Logging
    Related controls: [ITEM-0152], [ITEM-0163], [ITEM-0172], [ITEM-0212], [ITEM-0628].
    - REQ-0293 Employ [Assignment: organization-defined methods] for coordinating [Assignment: organization-defined audit information] among external organizations when audit information is transmitted across organizational boundaries.
    - ITEM-0203 AU-16(1) Identity Preservation
      Related controls: [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0379].
      - REQ-0294 Preserve the identity of individuals in cross-organizational audit trails.
    - ITEM-0204 AU-16(2) Sharing of Audit Information
      Related controls: [ITEM-0409], [ITEM-0918].
      - REQ-0295 Provide cross-organizational audit information to [Assignment: organization-defined organizations] based on [Assignment: organization-defined cross-organizational sharing agreements].
    - ITEM-0205 AU-16(3) Disassociability
      - REQ-0296 Implement [Assignment: organization-defined measures] to disassociate individuals from audit information transmitted across organizational boundaries.
- ITEM-0206 Assessment, Authorization, and Monitoring
  The assessment, authorization, and monitoring family (CA) of NIST SP 800-53 Revision 5.
  - ITEM-0207 CA-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0297 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] assessment, authorization, and monitoring policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the assessment, authorization, and monitoring policy and the associated assessment, authorization, and monitoring controls;
    - REQ-0298 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the assessment, authorization, and monitoring policy and procedures; and #low #moderate #high #privacy
    - REQ-0299 (c) Review and update the current assessment, authorization, and monitoring: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0208 CA-2 Control Assessments
    Related controls: [ITEM-0118], [ITEM-0215], [ITEM-0217], [ITEM-0220], [ITEM-0565], [ITEM-0641], [ITEM-0654], [ITEM-0726], [ITEM-0877], [ITEM-0913], [ITEM-0972], [ITEM-1009], [ITEM-1011].
    - REQ-0300 (a) Select the appropriate assessor or assessment team for the type of assessment to be conducted; #low #moderate #high #privacy
    - REQ-0301 (b) Develop a control assessment plan that describes the scope of the assessment including: #low #moderate #high #privacy
      (b)(1) Controls and control enhancements under assessment;
      (b)(2) Assessment procedures to be used to determine control effectiveness; and
      (b)(3) Assessment environment, assessment team, and assessment roles and responsibilities;
    - REQ-0302 (c) Ensure the control assessment plan is reviewed and approved by the authorizing official or designated representative prior to conducting the assessment; #low #moderate #high #privacy
    - REQ-0303 (d) Assess the controls in the system and its environment of operation [Assignment: organization-defined assessment frequency] to determine the extent to which the controls are implemented correctly, operating as intended, and producing the desired outcome with respect to meeting established security and privacy requirements; #low #moderate #high #privacy
    - REQ-0304 (e) Produce a control assessment report that document the results of the assessment; and #low #moderate #high #privacy
    - REQ-0305 (f) Provide the results of the control assessment to [Assignment: organization-defined individuals or roles]. #low #moderate #high #privacy
    - ITEM-0209 CA-2(1) Independent Assessors
      - REQ-0306 Employ independent assessors or assessment teams to conduct control assessments. #moderate #high
    - ITEM-0210 CA-2(2) Specialized Assessments
      Related controls: [ITEM-0496], [ITEM-0906].
      - REQ-0307 Include as part of control assessments, [Assignment: organization-defined specialized assessment frequency], [Selection: announced; unannounced], [Selection (one or more): in-depth monitoring; security instrumentation; automated security test cases; vulnerability scanning; malicious user testing; insider threat assessment; performance and load testing; data leakage or data loss assessment; [Assignment: organization-defined other forms of assessment] ]. #high
    - ITEM-0211 CA-2(3) Leveraging Results from External Organizations
      Related controls: [ITEM-0662].
      - REQ-0308 Leverage the results of control assessments performed by [Assignment: organization-defined external organization(s)] on [Assignment: organization-defined system] when the assessment meets [Assignment: organization-defined requirements].
  - ITEM-0212 CA-3 Information Exchange
    Related controls: [ITEM-0030], [ITEM-0118], [ITEM-0202], [ITEM-0217], [ITEM-0350], [ITEM-0409], [ITEM-0544], [ITEM-0628], [ITEM-0636], [ITEM-0709], [ITEM-0782], [ITEM-0972].
    - REQ-0309 (a) Approve and manage the exchange of information between the system and other systems using [Selection (one or more): interconnection security agreements; information exchange security agreements; memoranda of understanding or agreement; service level agreements; user agreements; non-disclosure agreements; [Assignment: organization-defined type of agreement] ]; #low #moderate #high
    - REQ-0310 (b) Document, as part of each exchange agreement, the interface characteristics, security and privacy requirements, controls, and responsibilities for each system, and the impact level of the information communicated; and #low #moderate #high
    - REQ-0311 (c) Review and update the agreements [Assignment: organization-defined frequency]. #low #moderate #high
    - ITEM-0213 CA-3(6) Transfer Authorizations
      Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0030].
      - REQ-0312 Verify that individuals or systems transferring data between interconnecting systems have the requisite authorizations (i.e., write permissions or privileges) prior to accepting such data. #high
    - ITEM-0214 CA-3(7) Transitive Information Exchanges
      Related controls: [ITEM-0782].
      - REQ-0313 (a) Identify transitive (downstream) information exchanges with other systems through the systems identified in [CA-3a](#ca-3_smt.a) ; and
      - REQ-0314 (b) Take measures to ensure that transitive (downstream) information exchanges cease when the controls on identified transitive (downstream) systems cannot be verified or validated.
  - ITEM-0215 CA-5 Plan of Action and Milestones
    Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0558], [ITEM-0565], [ITEM-0651], [ITEM-0906], [ITEM-0972].
    - REQ-0315 (a) Develop a plan of action and milestones for the system to document the planned remediation actions of the organization to correct weaknesses or deficiencies noted during the assessment of the controls and to reduce or eliminate known vulnerabilities in the system; and #low #moderate #high #privacy
    - REQ-0316 (b) Update existing plan of action and milestones [Assignment: organization-defined frequency] based on the findings from control assessments, independent audits or reviews, and continuous monitoring activities. #low #moderate #high #privacy
    - ITEM-0216 CA-5(1) Automation Support for Accuracy and Currency
      - REQ-0317 Ensure the accuracy, currency, and availability of the plan of action and milestones for the system using [Assignment: organization-defined automated mechanisms].
  - ITEM-0217 CA-6 Authorization
    Related controls: [ITEM-0208], [ITEM-0212], [ITEM-0220], [ITEM-0565], [ITEM-0566], [ITEM-0636], [ITEM-0718], [ITEM-0972].
    - REQ-0318 (a) Assign a senior official as the authorizing official for the system; #low #moderate #high #privacy
    - REQ-0319 (b) Assign a senior official as the authorizing official for common controls available for inheritance by organizational systems; #low #moderate #high #privacy
    - REQ-0320 (c) Ensure that the authorizing official for the system, before commencing operations: #low #moderate #high #privacy
      (c)(1) Accepts the use of common controls inherited by the system; and
      (c)(2) Authorizes the system to operate;
    - REQ-0321 (d) Ensure that the authorizing official for common controls authorizes the use of those controls for inheritance by organizational systems; #low #moderate #high #privacy
    - REQ-0322 (e) Update the authorizations [Assignment: organization-defined frequency]. #low #moderate #high #privacy
    - ITEM-0218 CA-6(1) Joint Authorization — Intra-organization
      Related controls: [ITEM-0062].
      - REQ-0323 Employ a joint authorization process for the system that includes multiple authorizing officials from the same organization conducting the authorization.
    - ITEM-0219 CA-6(2) Joint Authorization — Inter-organization
      Related controls: [ITEM-0062].
      - REQ-0324 Employ a joint authorization process for the system that includes multiple authorizing officials with at least one authorizing official from an organization external to the organization conducting the authorization.
  - ITEM-0220 CA-7 Continuous Monitoring
    Related controls: [ITEM-0003], [ITEM-0062], [ITEM-0102], [ITEM-0147], [ITEM-0163], [ITEM-0195], [ITEM-0208], [ITEM-0215], [ITEM-0217], [ITEM-0239], [ITEM-0248], [ITEM-0256], [ITEM-0282], [ITEM-0361], [ITEM-0425], [ITEM-0442], [ITEM-0444], [ITEM-0451], [ITEM-0496], [ITEM-0507], [ITEM-0528], [ITEM-0533], [ITEM-0538], [ITEM-0544], [ITEM-0558], [ITEM-0561], [ITEM-0565], [ITEM-0566], [ITEM-0568], [ITEM-0570], [ITEM-0581], [ITEM-0586], [ITEM-0590], [ITEM-0607], [ITEM-0628], [ITEM-0636], [ITEM-0641], [ITEM-0651], [ITEM-0654], [ITEM-0675], [ITEM-0709], [ITEM-0726], [ITEM-0777], [ITEM-0782], [ITEM-0833], [ITEM-0877], [ITEM-0892], [ITEM-0913], [ITEM-0918], [ITEM-0972], [ITEM-1023].
    - REQ-0325 (a) Establishing the following system-level metrics to be monitored: [Assignment: organization-defined system-level metrics]; #low #moderate #high #privacy
    - REQ-0326 (b) Establishing [Assignment: organization-defined frequencies] for monitoring and [Assignment: organization-defined frequencies] for assessment of control effectiveness; #low #moderate #high #privacy
    - REQ-0327 (c) Ongoing control assessments in accordance with the continuous monitoring strategy; #low #moderate #high #privacy
    - REQ-0328 (d) Ongoing monitoring of system and organization-defined metrics in accordance with the continuous monitoring strategy; #low #moderate #high #privacy
    - REQ-0329 (e) Correlation and analysis of information generated by control assessments and monitoring; #low #moderate #high #privacy
    - REQ-0330 (f) Response actions to address results of the analysis of control assessment and monitoring information; and #low #moderate #high #privacy
    - REQ-0331 (g) Reporting the security and privacy status of the system to [Assignment: organization-defined personnel or roles] [Assignment: organization-defined frequency]. #low #moderate #high #privacy
    - ITEM-0221 CA-7(1) Independent Assessment
      - REQ-0332 Employ independent assessors or assessment teams to monitor the controls in the system on an ongoing basis. #moderate #high
    - ITEM-0222 CA-7(3) Trend Analyses
      - REQ-0333 Employ trend analyses to determine if control implementations, the frequency of continuous monitoring activities, and the types of activities used in the continuous monitoring process need to be modified based on empirical data.
    - ITEM-0223 CA-7(4) Risk Monitoring
      - REQ-0334 (a) Effectiveness monitoring; #low #moderate #high #privacy
      - REQ-0335 (b) Compliance monitoring; and #low #moderate #high #privacy
      - REQ-0336 (c) Change monitoring. #low #moderate #high #privacy
    - ITEM-0224 CA-7(5) Consistency Analysis
      - REQ-0337 Employ the following actions to validate that policies are established and implemented controls are operating in a consistent manner: [Assignment: organization-defined actions].
    - ITEM-0225 CA-7(6) Automation Support for Monitoring
      - REQ-0338 Ensure the accuracy, currency, and availability of monitoring results for the system using [Assignment: organization-defined automated mechanisms].
  - ITEM-0226 CA-8 Penetration Testing
    Related controls: [ITEM-0641], [ITEM-0654], [ITEM-0726], [ITEM-1020], [ITEM-1023].
    - REQ-0339 Conduct penetration testing [Assignment: organization-defined frequency] on [Assignment: organization-defined system(s) or system components]. #high
    - ITEM-0227 CA-8(1) Independent Penetration Testing Agent or Team
      Related controls: [ITEM-0208].
      - REQ-0340 Employ an independent penetration testing agent or team to perform penetration testing on the system or system components. #high
    - ITEM-0228 CA-8(2) Red Team Exercises
      - REQ-0341 Employ the following red-team exercises to simulate attempts by adversaries to compromise organizational systems in accordance with applicable rules of engagement: [Assignment: organization-defined red team exercises].
    - ITEM-0229 CA-8(3) Facility Penetration Testing
      Related controls: [ITEM-0208], [ITEM-0496].
      - REQ-0342 Employ a penetration testing process that includes [Assignment: organization-defined frequency] [Selection (one or more): announced; unannounced] attempts to bypass or circumvent controls associated with physical access points to the facility.
  - ITEM-0230 CA-9 Internal System Connections
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0110], [ITEM-0115], [ITEM-0234], [ITEM-0350], [ITEM-0782], [ITEM-0972].
    - REQ-0343 (a) Authorize internal connections of [Assignment: organization-defined system components] to the system; #low #moderate #high
    - REQ-0344 (b) Document, for each internal connection, the interface characteristics, security and privacy requirements, and the nature of the information communicated; #low #moderate #high
    - REQ-0345 (c) Terminate internal system connections after [Assignment: organization-defined conditions] ; and #low #moderate #high
    - REQ-0346 (d) Review [Assignment: organization-defined frequency] the continued need for each internal connection. #low #moderate #high
    - ITEM-0231 CA-9(1) Compliance Checks
      Related controls: [ITEM-0256].
      - REQ-0347 Perform security and privacy compliance checks on constituent system components prior to the establishment of the internal connection.
- ITEM-0232 Configuration Management
  The configuration management family (CM) of NIST SP 800-53 Revision 5.
  - ITEM-0233 CM-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0675], [ITEM-0972].
    - REQ-0348 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] configuration management policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the configuration management policy and the associated configuration management controls;
    - REQ-0349 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the configuration management policy and procedures; and #low #moderate #high #privacy
    - REQ-0350 (c) Review and update the current configuration management: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0234 CM-2 Baseline Configuration
    Related controls: [ITEM-0115], [ITEM-0163], [ITEM-0230], [ITEM-0233], [ITEM-0239], [ITEM-0251], [ITEM-0256], [ITEM-0269], [ITEM-0278], [ITEM-0324], [ITEM-0332], [ITEM-0337], [ITEM-0442], [ITEM-0548], [ITEM-0559], [ITEM-0675], [ITEM-0718], [ITEM-0736], [ITEM-0833].
    - REQ-0351 (a) Develop, document, and maintain under configuration control, a current baseline configuration of the system; and #low #moderate #high
    - REQ-0352 (b) Review and update the baseline configuration of the system: #low #moderate #high
      (b)(1) [Assignment: organization-defined frequency];
      (b)(2) When required due to [Assignment: organization-defined circumstances] ; and
      (b)(3) When system components are installed or upgraded.
    - ITEM-0235 CM-2(2) Automation Support for Accuracy and Currency
      Related controls: [ITEM-0259], [ITEM-0350], [ITEM-0641].
      - REQ-0353 Maintain the currency, completeness, accuracy, and availability of the baseline configuration of the system using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0236 CM-2(3) Retention of Previous Configurations
      - REQ-0354 Retain [Assignment: organization-defined number] of previous versions of baseline configurations of the system to support rollback. #moderate #high
    - ITEM-0237 CM-2(6) Development and Test Environments
      Related controls: [ITEM-0248], [ITEM-0769], [ITEM-0782].
      - REQ-0355 Maintain a baseline configuration for system development and test environments that is managed separately from the operational baseline configuration.
    - ITEM-0238 CM-2(7) Configure Systems and Components for High-risk Areas
      Related controls: [ITEM-0473], [ITEM-0475].
      - REQ-0356 (a) Issue [Assignment: organization-defined systems or system components] with [Assignment: organization-defined configurations] to individuals traveling to locations that the organization deems to be of significant risk; and #moderate #high
      - REQ-0357 (b) Apply the following controls to the systems or components when the individuals return from travel: [Assignment: organization-defined controls]. #moderate #high
  - ITEM-0239 CM-3 Configuration Change Control
    Related controls: [ITEM-0220], [ITEM-0234], [ITEM-0248], [ITEM-0251], [ITEM-0256], [ITEM-0278], [ITEM-0282], [ITEM-0350], [ITEM-0442], [ITEM-0533], [ITEM-0625], [ITEM-0652], [ITEM-0675], [ITEM-0718], [ITEM-0851], [ITEM-0868], [ITEM-0875], [ITEM-0906], [ITEM-0913], [ITEM-0918], [ITEM-0947], [ITEM-0964], [ITEM-1030].
    - REQ-0358 (a) Determine and document the types of changes to the system that are configuration-controlled; #moderate #high
    - REQ-0359 (b) Review proposed configuration-controlled changes to the system and approve or disapprove such changes with explicit consideration for security and privacy impact analyses; #moderate #high
    - REQ-0360 (c) Document configuration change decisions associated with the system; #moderate #high
    - REQ-0361 (d) Implement approved configuration-controlled changes to the system; #moderate #high
    - REQ-0362 (e) Retain records of configuration-controlled changes to the system for [Assignment: organization-defined time period]; #moderate #high
    - REQ-0363 (f) Monitor and review activities associated with configuration-controlled changes to the system; and #moderate #high
    - REQ-0364 (g) Coordinate and provide oversight for configuration change control activities through [Assignment: organization-defined configuration change control element] that convenes [Selection (one or more): [Assignment: organization-defined frequency] ; when [Assignment: organization-defined configuration change conditions] ]. #moderate #high
    - ITEM-0240 CM-3(1) Automated Documentation, Notification, and Prohibition of Changes
      - REQ-0365 (a) Document proposed changes to the system; #high
      - REQ-0366 (b) Notify [Assignment: organization-defined approval authorities] of proposed changes to the system and request change approval; #high
      - REQ-0367 (c) Highlight proposed changes to the system that have not been approved or disapproved within [Assignment: organization-defined time period]; #high
      - REQ-0368 (d) Prohibit changes to the system until designated approvals are received; #high
      - REQ-0369 (e) Document all changes to the system; and #high
      - REQ-0370 (f) Notify [Assignment: organization-defined personnel] when approved changes to the system are completed. #high
    - ITEM-0241 CM-3(2) Testing, Validation, and Documentation of Changes
      - REQ-0371 Test, validate, and document changes to the system before finalizing the implementation of the changes. #moderate #high
    - ITEM-0242 CM-3(3) Automated Change Implementation
      - REQ-0372 Implement changes to the current system baseline and deploy the updated baseline across the installed base using [Assignment: organization-defined automated mechanisms].
    - ITEM-0243 CM-3(4) Security and Privacy Representatives
      - REQ-0373 Require [Assignment: organization-defined security and privacy representatives] to be members of the [Assignment: organization-defined configuration change control element]. #moderate #high
    - ITEM-0244 CM-3(5) Automated Security Response
      - REQ-0374 Implement the following security responses automatically if baseline configurations are changed in an unauthorized manner: [Assignment: organization-defined security responses].
    - ITEM-0245 CM-3(6) Cryptography Management
      Related controls: [ITEM-0818].
      - REQ-0375 Ensure that cryptographic mechanisms used to provide the following controls are under configuration management: [Assignment: organization-defined controls]. #high
    - ITEM-0246 CM-3(7) Review System Changes
      Related controls: [ITEM-0163], [ITEM-0172], [ITEM-0239].
      - REQ-0376 Review changes to the system [Assignment: organization-defined frequency] or when [Assignment: organization-defined circumstances] to determine whether unauthorized changes have occurred.
    - ITEM-0247 CM-3(8) Prevent or Restrict Configuration Changes
      - REQ-0377 Prevent or restrict changes to the configuration of the system under the following circumstances: [Assignment: organization-defined circumstances].
  - ITEM-0248 CM-4 Impact Analyses
    Related controls: [ITEM-0220], [ITEM-0239], [ITEM-0269], [ITEM-0278], [ITEM-0442], [ITEM-0636], [ITEM-0641], [ITEM-0652], [ITEM-0674], [ITEM-0675], [ITEM-0718], [ITEM-0906].
    - REQ-0378 Analyze changes to the system to determine potential security and privacy impacts prior to change implementation. #low #moderate #high #privacy
    - ITEM-0249 CM-4(1) Separate Test Environments
      Related controls: [ITEM-0726], [ITEM-0782].
      - REQ-0379 Analyze changes to the system in a separate test environment before implementation in an operational environment, looking for security and privacy impacts due to flaws, weaknesses, incompatibility, or intentional malice. #high
    - ITEM-0250 CM-4(2) Verification of Controls
      Related controls: [ITEM-0726], [ITEM-0769], [ITEM-0944].
      - REQ-0380 After system changes, verify that the impacted controls are implemented correctly, operating as intended, and producing the desired outcome with regard to meeting the security and privacy requirements for the system. #moderate #high
  - ITEM-0251 CM-5 Access Restrictions for Change
    Related controls: [ITEM-0016], [ITEM-0061], [ITEM-0062], [ITEM-0278], [ITEM-0496], [ITEM-0851], [ITEM-0868], [ITEM-0875], [ITEM-0906], [ITEM-0964].
    - REQ-0381 Define, document, approve, and enforce physical and logical access restrictions associated with changes to the system. #low #moderate #high
    - ITEM-0252 CM-5(1) Automated Access Enforcement and Audit Records
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0172], [ITEM-0190], [ITEM-0256], [ITEM-0282], [ITEM-0972].
      - REQ-0382 (a) Enforce access restrictions using [Assignment: organization-defined automated mechanisms] ; and #high
      - REQ-0383 (b) Automatically generate audit records of the enforcement actions. #high
    - ITEM-0253 CM-5(4) Dual Authorization
      Related controls: [ITEM-0003], [ITEM-0061], [ITEM-0239].
      - REQ-0384 Enforce dual authorization for implementing changes to [Assignment: organization-defined system components and system-level information].
    - ITEM-0254 CM-5(5) Privilege Limitation for Production and Operation
      Related controls: [ITEM-0003].
      - REQ-0385 (a) Limit privileges to change system components and system-related information within a production or operational environment; and
      - REQ-0386 (b) Review and reevaluate privileges [Assignment: organization-defined frequency].
    - ITEM-0255 CM-5(6) Limit Library Privileges
      Related controls: [ITEM-0003].
      - REQ-0387 Limit privileges to change software resident within software libraries.
  - ITEM-0256 CM-6 Configuration Settings
    Related controls: [ITEM-0016], [ITEM-0115], [ITEM-0151], [ITEM-0163], [ITEM-0230], [ITEM-0234], [ITEM-0239], [ITEM-0251], [ITEM-0259], [ITEM-0282], [ITEM-0312], [ITEM-0324], [ITEM-0332], [ITEM-0350], [ITEM-0361], [ITEM-0548], [ITEM-0551], [ITEM-0641], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0709], [ITEM-0833], [ITEM-0851], [ITEM-0892], [ITEM-0906], [ITEM-0918], [ITEM-0944].
    - REQ-0388 (a) Establish and document configuration settings for components employed within the system that reflect the most restrictive mode consistent with operational requirements using [Assignment: organization-defined common secure configurations]; #low #moderate #high
    - REQ-0389 (b) Implement the configuration settings; #low #moderate #high
    - REQ-0390 (c) Identify, document, and approve any deviations from established configuration settings for [Assignment: organization-defined system components] based on [Assignment: organization-defined operational requirements] ; and #low #moderate #high
    - REQ-0391 (d) Monitor and control changes to the configuration settings in accordance with organizational policies and procedures. #low #moderate #high
    - ITEM-0257 CM-6(1) Automated Management, Application, and Verification
      Related controls: [ITEM-0220].
      - REQ-0392 Manage, apply, and verify configuration settings for [Assignment: organization-defined system components] using [Assignment: organization-defined automated mechanisms]. #high
    - ITEM-0258 CM-6(2) Respond to Unauthorized Changes
      Related controls: [ITEM-0409], [ITEM-0427], [ITEM-0947].
      - REQ-0393 Take the following actions in response to unauthorized changes to [Assignment: organization-defined configuration settings]: [Assignment: organization-defined actions]. #high
  - ITEM-0259 CM-7 Least Functionality
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0234], [ITEM-0251], [ITEM-0256], [ITEM-0282], [ITEM-0641], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0709], [ITEM-0736], [ITEM-0766], [ITEM-0769], [ITEM-0782], [ITEM-0875], [ITEM-0918].
    - REQ-0394 (a) Configure the system to provide only [Assignment: organization-defined mission-essential capabilities] ; and #low #moderate #high
    - REQ-0395 (b) Prohibit or restrict the use of the following functions, ports, protocols, software, and/or services: [Assignment: organization-defined prohibited or restricted functions, system ports, protocols, software, and/or services]. #low #moderate #high
    - ITEM-0260 CM-7(1) Periodic Review
      Related controls: [ITEM-0110].
      - REQ-0396 (a) Review the system [Assignment: organization-defined frequency] to identify unnecessary and/or nonsecure functions, ports, protocols, software, and services; and #moderate #high
      - REQ-0397 (b) Disable or remove [Assignment: organization-defined functions, ports, protocols, software, and services within the system deemed to be unnecessary and/or nonsecure]. #moderate #high
    - ITEM-0261 CM-7(2) Prevent Program Execution
      Related controls: [ITEM-0269], [ITEM-0545], [ITEM-0551], [ITEM-0559], [ITEM-0604].
      - REQ-0398 Prevent program execution in accordance with [Selection (one or more): [Assignment: organization-defined policies, rules of behavior, and/or access agreements regarding software program usage and restrictions] ; rules authorizing the terms and conditions of software program usage]. #moderate #high
    - ITEM-0262 CM-7(3) Registration Compliance
      - REQ-0399 Ensure compliance with [Assignment: organization-defined registration requirements].
    - ITEM-0263 CM-7(4) Unauthorized Software — Deny-by-exception
      Related controls: [ITEM-0256], [ITEM-0269], [ITEM-0280], [ITEM-0551], [ITEM-0559].
      - REQ-0400 (a) Identify [Assignment: organization-defined software programs];
      - REQ-0401 (b) Employ an allow-all, deny-by-exception policy to prohibit the execution of unauthorized software programs on the system; and
      - REQ-0402 (c) Review and update the list of unauthorized software programs [Assignment: organization-defined frequency].
    - ITEM-0264 CM-7(5) Authorized Software — Allow-by-exception
      Related controls: [ITEM-0234], [ITEM-0256], [ITEM-0269], [ITEM-0280], [ITEM-0551], [ITEM-0559], [ITEM-0718], [ITEM-0868], [ITEM-0947].
      - REQ-0403 (a) Identify [Assignment: organization-defined software programs]; #moderate #high
      - REQ-0404 (b) Employ a deny-all, permit-by-exception policy to allow the execution of authorized software programs on the system; and #moderate #high
      - REQ-0405 (c) Review and update the list of authorized software programs [Assignment: organization-defined frequency]. #moderate #high
    - ITEM-0265 CM-7(6) Confined Environments with Limited Privileges
      Related controls: [ITEM-0282], [ITEM-0893].
      - REQ-0406 Require that the following user-installed software execute in a confined physical or virtual machine environment with limited privileges: [Assignment: organization-defined user-installed software].
    - ITEM-0266 CM-7(7) Code Execution in Protected Environments
      Related controls: [ITEM-0280], [ITEM-0893].
      - REQ-0407 (a) Obtained from sources with limited or no warranty; and/or
      - REQ-0408 (b) Without the provision of source code.
    - ITEM-0267 CM-7(8) Binary or Machine Executable Code
      Related controls: [ITEM-0674], [ITEM-0761].
      - REQ-0409 (a) Prohibit the use of binary or machine-executable code from sources with limited or no warranty or without the provision of source code; and
      - REQ-0410 (b) Allow exceptions only for compelling mission or operational requirements and with the approval of the authorizing official.
    - ITEM-0268 CM-7(9) Prohibiting The Use of Unauthorized Hardware
      - REQ-0411 (a) Identify [Assignment: organization-defined hardware components];
      - REQ-0412 (b) Prohibit the use or connection of unauthorized hardware components;
      - REQ-0413 (c) Review and update the list of authorized hardware components [Assignment: organization-defined frequency].
  - ITEM-0269 CM-8 System Component Inventory
    Related controls: [ITEM-0234], [ITEM-0259], [ITEM-0278], [ITEM-0280], [ITEM-0282], [ITEM-0287], [ITEM-0291], [ITEM-0324], [ITEM-0442], [ITEM-0464], [ITEM-0538], [ITEM-0551], [ITEM-0559], [ITEM-0662], [ITEM-0674], [ITEM-0906], [ITEM-1015].
    - REQ-0414 (a) Develop and document an inventory of system components that: #low #moderate #high
      (a)(1) Accurately reflects the system;
      (a)(2) Includes all components within the system;
      (a)(3) Does not include duplicate accounting of components or components assigned to any other system;
      (a)(4) Is at the level of granularity deemed necessary for tracking and reporting; and
      (a)(5) Includes the following information to achieve system component accountability: [Assignment: organization-defined information] ; and
    - REQ-0415 (b) Review and update the system component inventory [Assignment: organization-defined frequency]. #low #moderate #high
    - ITEM-0270 CM-8(1) Updates During Installation and Removal
      Related controls: [ITEM-0572].
      - REQ-0416 Update the inventory of system components as part of component installations, removals, and system updates. #moderate #high
    - ITEM-0271 CM-8(2) Automated Maintenance
      - REQ-0417 Maintain the currency, completeness, accuracy, and availability of the inventory of system components using [Assignment: organization-defined automated mechanisms]. #high
    - ITEM-0272 CM-8(3) Automated Unauthorized Component Detection
      Related controls: [ITEM-0115], [ITEM-0220], [ITEM-0641], [ITEM-0769], [ITEM-0878], [ITEM-0893], [ITEM-0913], [ITEM-0918], [ITEM-0947].
      - REQ-0418 (a) Detect the presence of unauthorized hardware, software, and firmware components within the system using [Assignment: organization-defined automated mechanisms] [Assignment: organization-defined frequency] ; and #moderate #high
      - REQ-0419 (b) Take the following actions when unauthorized components are detected: [Selection (one or more): disable network access by unauthorized components; isolate unauthorized components; notify [Assignment: organization-defined personnel or roles] ]. #moderate #high
    - ITEM-0273 CM-8(4) Accountability Information
      Related controls: [ITEM-0016].
      - REQ-0420 Include in the system component inventory information, a means for identifying by [Selection (one or more): name; position; role] , individuals responsible and accountable for administering those components. #high
    - ITEM-0274 CM-8(6) Assessed Configurations and Approved Deviations
      - REQ-0421 Include assessed component configurations and any approved deviations to current deployed configurations in the system component inventory.
    - ITEM-0275 CM-8(7) Centralized Repository
      - REQ-0422 Provide a centralized repository for the inventory of system components.
    - ITEM-0276 CM-8(8) Automated Location Tracking
      - REQ-0423 Support the tracking of system components by geographic location using [Assignment: organization-defined automated mechanisms].
    - ITEM-0277 CM-8(9) Assignment of Components to Systems
      - REQ-0424 (a) Assign system components to a system; and
      - REQ-0425 (b) Receive an acknowledgement from [Assignment: organization-defined personnel or roles] of this assignment.
  - ITEM-0278 CM-9 Configuration Management Plan
    Related controls: [ITEM-0234], [ITEM-0239], [ITEM-0248], [ITEM-0251], [ITEM-0269], [ITEM-0544], [ITEM-0652], [ITEM-0718], [ITEM-0972].
    - REQ-0426 (a) Addresses roles, responsibilities, and configuration management processes and procedures; #moderate #high
    - REQ-0427 (b) Establishes a process for identifying configuration items throughout the system development life cycle and for managing the configuration of the configuration items; #moderate #high
    - REQ-0428 (c) Defines the configuration items for the system and places the configuration items under configuration management; #moderate #high
    - REQ-0429 (d) Is reviewed and approved by [Assignment: organization-defined personnel or roles] ; and #moderate #high
    - REQ-0430 (e) Protects the configuration management plan from unauthorized disclosure and modification. #moderate #high
    - ITEM-0279 CM-9(1) Assignment of Responsibility
      - REQ-0431 Assign responsibility for developing the configuration management process to organizational personnel that are not directly involved in system development.
  - ITEM-0280 CM-10 Software Usage Restrictions
    Related controls: [ITEM-0102], [ITEM-0163], [ITEM-0259], [ITEM-0269], [ITEM-0588], [ITEM-0782].
    - REQ-0432 (a) Use software and associated documentation in accordance with contract agreements and copyright laws; #low #moderate #high
    - REQ-0433 (b) Track the use of software and associated documentation protected by quantity licenses to control copying and distribution; and #low #moderate #high
    - REQ-0434 (c) Control and document the use of peer-to-peer file sharing technology to ensure that this capability is not used for the unauthorized distribution, display, performance, or reproduction of copyrighted work. #low #moderate #high
    - ITEM-0281 CM-10(1) Open-source Software
      Related controls: [ITEM-0947].
      - REQ-0435 Establish the following restrictions on the use of open-source software: [Assignment: organization-defined restrictions].
  - ITEM-0282 CM-11 User-installed Software
    Related controls: [ITEM-0016], [ITEM-0163], [ITEM-0234], [ITEM-0239], [ITEM-0251], [ITEM-0256], [ITEM-0259], [ITEM-0269], [ITEM-0545], [ITEM-0918], [ITEM-0947].
    - REQ-0436 (a) Establish [Assignment: organization-defined policies] governing the installation of software by users; #low #moderate #high
    - REQ-0437 (b) Enforce software installation policies through the following methods: [Assignment: organization-defined methods] ; and #low #moderate #high
    - REQ-0438 (c) Monitor policy compliance [Assignment: organization-defined frequency]. #low #moderate #high
    - ITEM-0283 CM-11(2) Software Installation with Privileged Status
      Related controls: [ITEM-0061], [ITEM-0062].
      - REQ-0439 Allow user installation of software only with explicit privileged status.
    - ITEM-0284 CM-11(3) Automated Enforcement and Monitoring
      - REQ-0440 Enforce and monitor compliance with software installation policies using [Assignment: organization-defined automated mechanisms].
  - ITEM-0285 CM-12 Information Location
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0030], [ITEM-0062], [ITEM-0128], [ITEM-0269], [ITEM-0559], [ITEM-0634], [ITEM-0662], [ITEM-0675], [ITEM-0749], [ITEM-0775], [ITEM-0828], [ITEM-0851], [ITEM-0918], [ITEM-0947].
    - REQ-0441 (a) Identify and document the location of [Assignment: organization-defined information] and the specific system components on which the information is processed and stored; #moderate #high
    - REQ-0442 (b) Identify and document the users who have access to the system and system components where the information is processed and stored; and #moderate #high
    - REQ-0443 (c) Document changes to the location (i.e., system or system components) where the information is processed and stored. #moderate #high
    - ITEM-0286 CM-12(1) Automated Tools to Support Information Location
      - REQ-0444 Use automated tools to identify [Assignment: organization-defined information by information type] on [Assignment: organization-defined system components] to ensure controls are in place to protect organizational information and individual privacy. #moderate #high
  - ITEM-0287 CM-13 Data Action Mapping
    Related controls: [ITEM-0016], [ITEM-0248], [ITEM-0285], [ITEM-0559], [ITEM-0585], [ITEM-0612], [ITEM-0615], [ITEM-0636], [ITEM-0652].
    - REQ-0445 Develop and document a map of system data actions.
  - ITEM-0288 CM-14 Signed Components
    Related controls: [ITEM-0259], [ITEM-0818], [ITEM-0823], [ITEM-0947].
    - REQ-0446 Prevent the installation of [Assignment: organization-defined software and firmware components] without verification that the component has been digitally signed using a certificate that is recognized and approved by the organization.
- ITEM-0289 Contingency Planning
  The contingency planning family (CP) of NIST SP 800-53 Revision 5.
  - ITEM-0290 CP-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0447 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] contingency planning policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the contingency planning policy and the associated contingency planning controls;
    - REQ-0448 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the contingency planning policy and procedures; and #low #moderate #high
    - REQ-0449 (c) Review and update the current contingency planning: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0291 CP-2 Contingency Plan
    Related controls: [ITEM-0299], [ITEM-0302], [ITEM-0308], [ITEM-0312], [ITEM-0318], [ITEM-0324], [ITEM-0332], [ITEM-0336], [ITEM-0338], [ITEM-0409], [ITEM-0427], [ITEM-0434], [ITEM-0436], [ITEM-0464], [ITEM-0471], [ITEM-0473], [ITEM-0475], [ITEM-0544], [ITEM-0564], [ITEM-0567], [ITEM-0736], [ITEM-0759], [ITEM-0782], [ITEM-0843], [ITEM-0972].
    - REQ-0450 (a) Develop a contingency plan for the system that: #low #moderate #high
      (a)(1) Identifies essential mission and business functions and associated contingency requirements;
      (a)(2) Provides recovery objectives, restoration priorities, and metrics;
      (a)(3) Addresses contingency roles, responsibilities, assigned individuals with contact information;
      (a)(4) Addresses maintaining essential mission and business functions despite a system disruption, compromise, or failure;
      (a)(5) Addresses eventual, full system restoration without deterioration of the controls originally planned and implemented;
      (a)(6) Addresses the sharing of contingency information; and
      (a)(7) Is reviewed and approved by [Assignment: organization-defined personnel or roles];
    - REQ-0451 (b) Distribute copies of the contingency plan to [Assignment: organization-defined key contingency personnel (identified by name and/or by role) and organizational elements]; #low #moderate #high
    - REQ-0452 (c) Coordinate contingency planning activities with incident handling activities; #low #moderate #high
    - REQ-0453 (d) Review the contingency plan for the system [Assignment: organization-defined frequency]; #low #moderate #high
    - REQ-0454 (e) Update the contingency plan to address changes to the organization, system, or environment of operation and problems encountered during contingency plan implementation, execution, or testing; #low #moderate #high
    - REQ-0455 (f) Communicate contingency plan changes to [Assignment: organization-defined key contingency personnel (identified by name and/or by role) and organizational elements]; #low #moderate #high
    - REQ-0456 (g) Incorporate lessons learned from contingency plan testing, training, or actual contingency activities into contingency testing and training; and #low #moderate #high
    - REQ-0457 (h) Protect the contingency plan from unauthorized disclosure and modification. #low #moderate #high
    - ITEM-0292 CP-2(1) Coordinate with Related Plans
      - REQ-0458 Coordinate contingency plan development with organizational elements responsible for related plans. #moderate #high
    - ITEM-0293 CP-2(2) Capacity Planning
      Related controls: [ITEM-0519], [ITEM-0522], [ITEM-0524], [ITEM-0528], [ITEM-0535], [ITEM-0777].
      - REQ-0459 Conduct capacity planning so that necessary capacity for information processing, telecommunications, and environmental support exists during contingency operations. #high
    - ITEM-0294 CP-2(3) Resume Mission and Business Functions
      - REQ-0460 Plan for the resumption of [Selection: all; essential] mission and business functions within [Assignment: organization-defined time period] of contingency plan activation. #moderate #high
    - ITEM-0295 CP-2(5) Continue Mission and Business Functions
      - REQ-0461 Plan for the continuance of [Selection: all; essential] mission and business functions with minimal or no loss of operational continuity and sustains that continuity until full system restoration at primary processing and/or storage sites. #high
    - ITEM-0296 CP-2(6) Alternate Processing and Storage Sites
      - REQ-0462 Plan for the transfer of [Selection: all; essential] mission and business functions to alternate processing and/or storage sites with minimal or no loss of operational continuity and sustain that continuity through system restoration to primary processing and/or storage sites.
    - ITEM-0297 CP-2(7) Coordinate with External Service Providers
      Related controls: [ITEM-0709].
      - REQ-0463 Coordinate the contingency plan with the contingency plans of external service providers to ensure that contingency requirements can be satisfied.
    - ITEM-0298 CP-2(8) Identify Critical Assets
      Related controls: [ITEM-0269], [ITEM-0653].
      - REQ-0464 Identify critical system assets supporting [Selection: all; essential] mission and business functions. #moderate #high
  - ITEM-0299 CP-3 Contingency Training
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0147], [ITEM-0291], [ITEM-0302], [ITEM-0318], [ITEM-0401], [ITEM-0409], [ITEM-0436].
    - REQ-0465 (a) Provide contingency training to system users consistent with assigned roles and responsibilities: #low #moderate #high
      (a)(1) Within [Assignment: organization-defined time period] of assuming a contingency role or responsibility;
      (a)(2) When required by system changes; and
      (a)(3) [Assignment: organization-defined frequency] thereafter; and
    - REQ-0466 (b) Review and update contingency training content [Assignment: organization-defined frequency] and following [Assignment: organization-defined events]. #low #moderate #high
    - ITEM-0300 CP-3(1) Simulated Events
      - REQ-0467 Incorporate simulated events into contingency training to facilitate effective response by personnel in crisis situations. #high
    - ITEM-0301 CP-3(2) Mechanisms Used in Training Environments
      - REQ-0468 Employ mechanisms used in operations to provide a more thorough and realistic contingency training environment.
  - ITEM-0302 CP-4 Contingency Plan Testing
    Related controls: [ITEM-0142], [ITEM-0291], [ITEM-0299], [ITEM-0318], [ITEM-0324], [ITEM-0405], [ITEM-0409], [ITEM-0544], [ITEM-0570], [ITEM-1009].
    - REQ-0469 (a) Test the contingency plan for the system [Assignment: organization-defined frequency] using the following tests to determine the effectiveness of the plan and the readiness to execute the plan: [Assignment: organization-defined tests]. #low #moderate #high
    - REQ-0470 (b) Review the contingency plan test results; and #low #moderate #high
    - REQ-0471 (c) Initiate corrective actions, if needed. #low #moderate #high
    - ITEM-0303 CP-4(1) Coordinate with Related Plans
      Related controls: [ITEM-0434], [ITEM-0564].
      - REQ-0472 Coordinate contingency plan testing with organizational elements responsible for related plans. #moderate #high
    - ITEM-0304 CP-4(2) Alternate Processing Site
      Related controls: [ITEM-0312].
      - REQ-0473 (a) To familiarize contingency personnel with the facility and available resources; and #high
      - REQ-0474 (b) To evaluate the capabilities of the alternate processing site to support contingency operations. #high
    - ITEM-0305 CP-4(3) Automated Testing
      - REQ-0475 Test the contingency plan using [Assignment: organization-defined automated mechanisms].
    - ITEM-0306 CP-4(4) Full Recovery and Reconstitution
      Related controls: [ITEM-0332], [ITEM-0847].
      - REQ-0476 Include a full recovery and reconstitution of the system to a known state as part of contingency plan testing.
    - ITEM-0307 CP-4(5) Self-challenge
      - REQ-0477 Employ [Assignment: organization-defined mechanisms] to [Assignment: organization-defined system or system component] to disrupt and adversely affect the system or system component.
  - ITEM-0308 CP-6 Alternate Storage Site
    Related controls: [ITEM-0291], [ITEM-0312], [ITEM-0318], [ITEM-0324], [ITEM-0332], [ITEM-0473], [ITEM-0475], [ITEM-0496], [ITEM-0872], [ITEM-0976].
    - REQ-0478 (a) Establish an alternate storage site, including necessary agreements to permit the storage and retrieval of system backup information; and #moderate #high
    - REQ-0479 (b) Ensure that the alternate storage site provides controls equivalent to that of the primary site. #moderate #high
    - ITEM-0309 CP-6(1) Separation from Primary Site
      Related controls: [ITEM-0636].
      - REQ-0480 Identify an alternate storage site that is sufficiently separated from the primary storage site to reduce susceptibility to the same threats. #moderate #high
    - ITEM-0310 CP-6(2) Recovery Time and Recovery Point Objectives
      - REQ-0481 Configure the alternate storage site to facilitate recovery operations in accordance with recovery time and recovery point objectives. #high
    - ITEM-0311 CP-6(3) Accessibility
      Related controls: [ITEM-0636].
      - REQ-0482 Identify potential accessibility problems to the alternate storage site in the event of an area-wide disruption or disaster and outline explicit mitigation actions. #moderate #high
  - ITEM-0312 CP-7 Alternate Processing Site
    Related controls: [ITEM-0291], [ITEM-0308], [ITEM-0318], [ITEM-0324], [ITEM-0332], [ITEM-0464], [ITEM-0496], [ITEM-0519], [ITEM-0522], [ITEM-0534], [ITEM-0872], [ITEM-0976].
    - REQ-0483 (a) Establish an alternate processing site, including necessary agreements to permit the transfer and resumption of [Assignment: organization-defined system operations] for essential mission and business functions within [Assignment: organization-defined time period] when the primary processing capabilities are unavailable; #moderate #high
    - REQ-0484 (b) Make available at the alternate processing site, the equipment and supplies required to transfer and resume operations or put contracts in place to support delivery to the site within the organization-defined time period for transfer and resumption; and #moderate #high
    - REQ-0485 (c) Provide controls at the alternate processing site that are equivalent to those at the primary site. #moderate #high
    - ITEM-0313 CP-7(1) Separation from Primary Site
      Related controls: [ITEM-0636].
      - REQ-0486 Identify an alternate processing site that is sufficiently separated from the primary processing site to reduce susceptibility to the same threats. #moderate #high
    - ITEM-0314 CP-7(2) Accessibility
      Related controls: [ITEM-0636].
      - REQ-0487 Identify potential accessibility problems to alternate processing sites in the event of an area-wide disruption or disaster and outlines explicit mitigation actions. #moderate #high
    - ITEM-0315 CP-7(3) Priority of Service
      - REQ-0488 Develop alternate processing site agreements that contain priority-of-service provisions in accordance with availability requirements (including recovery time objectives). #moderate #high
    - ITEM-0316 CP-7(4) Preparation for Use
      Related controls: [ITEM-0234], [ITEM-0256], [ITEM-0302].
      - REQ-0489 Prepare the alternate processing site so that the site can serve as the operational site supporting essential mission and business functions. #high
    - ITEM-0317 CP-7(6) Inability to Return to Primary Site
      - REQ-0490 Plan and prepare for circumstances that preclude returning to the primary processing site.
  - ITEM-0318 CP-8 Telecommunications Services
    Related controls: [ITEM-0291], [ITEM-0308], [ITEM-0312], [ITEM-0336], [ITEM-0782].
    - REQ-0491 Establish alternate telecommunications services, including necessary agreements to permit the resumption of [Assignment: organization-defined system operations] for essential mission and business functions within [Assignment: organization-defined time period] when the primary telecommunications capabilities are unavailable at either the primary or alternate processing or storage sites. #moderate #high
    - ITEM-0319 CP-8(1) Priority of Service Provisions
      - REQ-0492 (a) Develop primary and alternate telecommunications service agreements that contain priority-of-service provisions in accordance with availability requirements (including recovery time objectives); and #moderate #high
      - REQ-0493 (b) Request Telecommunications Service Priority for all telecommunications services used for national security emergency preparedness if the primary and/or alternate telecommunications services are provided by a common carrier. #moderate #high
    - ITEM-0320 CP-8(2) Single Points of Failure
      - REQ-0494 Obtain alternate telecommunications services to reduce the likelihood of sharing a single point of failure with primary telecommunications services. #moderate #high
    - ITEM-0321 CP-8(3) Separation of Primary and Alternate Providers
      - REQ-0495 Obtain alternate telecommunications services from providers that are separated from primary service providers to reduce susceptibility to the same threats. #high
    - ITEM-0322 CP-8(4) Provider Contingency Plan
      Related controls: [ITEM-0299], [ITEM-0302].
      - REQ-0496 (a) Require primary and alternate telecommunications service providers to have contingency plans; #high
      - REQ-0497 (b) Review provider contingency plans to ensure that the plans meet organizational contingency requirements; and #high
      - REQ-0498 (c) Obtain evidence of contingency testing and training by providers [Assignment: organization-defined frequency]. #high
    - ITEM-0323 CP-8(5) Alternate Telecommunication Service Testing
      Related controls: [ITEM-0299].
      - REQ-0499 Test alternate telecommunication services [Assignment: organization-defined frequency].
  - ITEM-0324 CP-9 System Backup
    Related controls: [ITEM-0291], [ITEM-0308], [ITEM-0332], [ITEM-0473], [ITEM-0475], [ITEM-0809], [ITEM-0818], [ITEM-0823], [ITEM-0918], [ITEM-0976].
    - REQ-0500 (a) Conduct backups of user-level information contained in [Assignment: organization-defined system components] [Assignment: organization-defined frequency]; #low #moderate #high
    - REQ-0501 (b) Conduct backups of system-level information contained in the system [Assignment: organization-defined frequency]; #low #moderate #high
    - REQ-0502 (c) Conduct backups of system documentation, including security- and privacy-related documentation [Assignment: organization-defined frequency] ; and #low #moderate #high
    - REQ-0503 (d) Protect the confidentiality, integrity, and availability of backup information. #low #moderate #high
    - ITEM-0325 CP-9(1) Testing for Reliability and Integrity
      Related controls: [ITEM-0302].
      - REQ-0504 Test backup information [Assignment: organization-defined frequency] to verify media reliability and information integrity. #moderate #high
    - ITEM-0326 CP-9(2) Test Restoration Using Sampling
      Related controls: [ITEM-0302].
      - REQ-0505 Use a sample of backup information in the restoration of selected system functions as part of contingency plan testing. #high
    - ITEM-0327 CP-9(3) Separate Storage for Critical Information
      Related controls: [ITEM-0234], [ITEM-0256], [ITEM-0269].
      - REQ-0506 Store backup copies of [Assignment: organization-defined critical system software and other security-related information] in a separate facility or in a fire rated container that is not collocated with the operational system. #high
    - ITEM-0328 CP-9(5) Transfer to Alternate Storage Site
      Related controls: [ITEM-0312], [ITEM-0472], [ITEM-0473], [ITEM-0475].
      - REQ-0507 Transfer system backup information to the alternate storage site [Assignment: organization-defined time period and transfer rate consistent with the recovery time and recovery point objectives]. #high
    - ITEM-0329 CP-9(6) Redundant Secondary System
      Related controls: [ITEM-0312].
      - REQ-0508 Conduct system backup by maintaining a redundant secondary system that is not collocated with the primary system and that can be activated without loss of information or disruption to operations.
    - ITEM-0330 CP-9(7) Dual Authorization for Deletion or Destruction
      Related controls: [ITEM-0016], [ITEM-0061], [ITEM-0471].
      - REQ-0509 Enforce dual authorization for the deletion or destruction of [Assignment: organization-defined backup information].
    - ITEM-0331 CP-9(8) Cryptographic Protection
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0851].
      - REQ-0510 Implement cryptographic mechanisms to prevent unauthorized disclosure and modification of [Assignment: organization-defined backup information]. #moderate #high
  - ITEM-0332 CP-10 System Recovery and Reconstitution
    Related controls: [ITEM-0291], [ITEM-0302], [ITEM-0308], [ITEM-0312], [ITEM-0324], [ITEM-0409], [ITEM-0675], [ITEM-0847], [ITEM-0976].
    - REQ-0511 Provide for the recovery and reconstitution of the system to a known state within [Assignment: organization-defined time period consistent with recovery time and recovery point objectives] after a disruption, compromise, or failure. #low #moderate #high
    - ITEM-0333 CP-10(2) Transaction Recovery
      - REQ-0512 Implement transaction recovery for systems that are transaction-based. #moderate #high
    - ITEM-0334 CP-10(4) Restore Within Time Period
      Related controls: [ITEM-0234], [ITEM-0256].
      - REQ-0513 Provide the capability to restore system components within [Assignment: organization-defined restoration time periods] from configuration-controlled and integrity-protected information representing a known, operational state for the components. #high
    - ITEM-0335 CP-10(6) Component Protection
      Related controls: [ITEM-0016], [ITEM-0062], [ITEM-0471], [ITEM-0473], [ITEM-0496], [ITEM-0507].
      - REQ-0514 Protect system components used for recovery and reconstitution.
  - ITEM-0336 CP-11 Alternate Communications Protocols
    Related controls: [ITEM-0291], [ITEM-0318], [ITEM-0338].
    - REQ-0515 Provide the capability to employ [Assignment: organization-defined alternative communications protocols] in support of maintaining continuity of operations.
  - ITEM-0337 CP-12 Safe Mode
    Related controls: [ITEM-0234], [ITEM-0675], [ITEM-0847], [ITEM-0976], [ITEM-0987].
    - REQ-0516 When [Assignment: organization-defined conditions] are detected, enter a safe mode of operation with [Assignment: organization-defined restrictions].
  - ITEM-0338 CP-13 Alternative Security Mechanisms
    Related controls: [ITEM-0291], [ITEM-0336], [ITEM-0976].
    - REQ-0517 Employ [Assignment: organization-defined alternative or supplemental security mechanisms] for satisfying [Assignment: organization-defined security functions] when the primary means of implementing the security function is unavailable or compromised.
- ITEM-0339 Identification and Authentication
  The identification and authentication family (IA) of NIST SP 800-53 Revision 5.
  - ITEM-0340 IA-1 Policy and Procedures
    Related controls: [ITEM-0002], [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0518 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] identification and authentication policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the identification and authentication policy and the associated identification and authentication controls;
    - REQ-0519 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the identification and authentication policy and procedures; and #low #moderate #high
    - REQ-0520 (c) Review and update the current identification and authentication: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0341 IA-2 Identification and Authentication (Organizational Users)
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0030], [ITEM-0090], [ITEM-0102], [ITEM-0110], [ITEM-0150], [ITEM-0163], [ITEM-0354], [ITEM-0361], [ITEM-0379], [ITEM-0395], [ITEM-0451], [ITEM-0458], [ITEM-0492], [ITEM-0545], [ITEM-0662], [ITEM-0675].
    - REQ-0521 Uniquely identify and authenticate organizational users and associate that unique identification with processes acting on behalf of those users. #low #moderate #high
    - ITEM-0342 IA-2(1) Multi-factor Authentication to Privileged Accounts
      Related controls: [ITEM-0061], [ITEM-0062].
      - REQ-0522 Implement multi-factor authentication for access to privileged accounts. #low #moderate #high
    - ITEM-0343 IA-2(2) Multi-factor Authentication to Non-privileged Accounts
      Related controls: [ITEM-0061].
      - REQ-0523 Implement multi-factor authentication for access to non-privileged accounts. #low #moderate #high
    - ITEM-0344 IA-2(5) Individual Authentication with Group Authentication
      - REQ-0524 When shared accounts or authenticators are employed, require users to be individually authenticated before granting access to the shared accounts or resources. #high
    - ITEM-0345 IA-2(6) Access to Accounts —separate Device
      Related controls: [ITEM-0062].
      - REQ-0525 (a) One of the factors is provided by a device separate from the system gaining access; and
      - REQ-0526 (b) The device meets [Assignment: organization-defined strength of mechanism requirements].
    - ITEM-0346 IA-2(8) Access to Accounts — Replay Resistant
      - REQ-0527 Implement replay-resistant authentication mechanisms for access to [Selection (one or more): privileged accounts; non-privileged accounts]. #low #moderate #high
    - ITEM-0347 IA-2(10) Single Sign-on
      - REQ-0528 Provide a single sign-on capability for [Assignment: organization-defined system accounts and services].
    - ITEM-0348 IA-2(12) Acceptance of PIV Credentials
      - REQ-0529 Accept and electronically verify Personal Identity Verification-compliant credentials. #low #moderate #high
    - ITEM-0349 IA-2(13) Out-of-band Authentication
      Related controls: [ITEM-0386], [ITEM-0387], [ITEM-0875].
      - REQ-0530 Implement the following out-of-band authentication mechanisms under [Assignment: organization-defined conditions]: [Assignment: organization-defined out-of-band authentication].
  - ITEM-0350 IA-3 Device Identification and Authentication
    Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0163], [ITEM-0212], [ITEM-0230], [ITEM-0354], [ITEM-0361], [ITEM-0385], [ITEM-0387], [ITEM-0395], [ITEM-0918].
    - REQ-0531 Uniquely identify and authenticate [Assignment: organization-defined devices and/or types of devices] before establishing a [Selection (one or more): local; remote; network] connection. #moderate #high
    - ITEM-0351 IA-3(1) Cryptographic Bidirectional Authentication
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-0532 Authenticate [Assignment: organization-defined devices and/or types of devices] before establishing [Selection (one or more): local; remote; network] connection using bidirectional authentication that is cryptographically based.
    - ITEM-0352 IA-3(3) Dynamic Address Allocation
      Related controls: [ITEM-0151].
      - REQ-0533 (a) Where addresses are allocated dynamically, standardize dynamic address allocation lease information and the lease duration assigned to devices in accordance with [Assignment: organization-defined lease information and lease duration] ; and
      - REQ-0534 (b) Audit lease information when assigned to a device.
    - ITEM-0353 IA-3(4) Device Attestation
      Related controls: [ITEM-0234], [ITEM-0239], [ITEM-0256].
      - REQ-0535 Handle device identification and authentication based on attestation by [Assignment: organization-defined configuration management process].
  - ITEM-0354 IA-4 Identifier Management
    Related controls: [ITEM-0061], [ITEM-0341], [ITEM-0350], [ITEM-0361], [ITEM-0379], [ITEM-0385], [ITEM-0388], [ITEM-0451], [ITEM-0492], [ITEM-0496], [ITEM-0504], [ITEM-0545], [ITEM-0568], [ITEM-0595], [ITEM-0600], [ITEM-0603], [ITEM-0875].
    - REQ-0536 (a) Receiving authorization from [Assignment: organization-defined personnel or roles] to assign an individual, group, role, service, or device identifier; #low #moderate #high
    - REQ-0537 (b) Selecting an identifier that identifies an individual, group, role, service, or device; #low #moderate #high
    - REQ-0538 (c) Assigning the identifier to the intended individual, group, role, service, or device; and #low #moderate #high
    - REQ-0539 (d) Preventing reuse of identifiers for [Assignment: organization-defined time period]. #low #moderate #high
    - ITEM-0355 IA-4(1) Prohibit Account Identifiers as Public Identifiers
      Related controls: [ITEM-0135], [ITEM-0628].
      - REQ-0540 Prohibit the use of system account identifiers that are the same as public identifiers for individual accounts.
    - ITEM-0356 IA-4(4) Identify User Status
      - REQ-0541 Manage individual identifiers by uniquely identifying each individual as [Assignment: organization-defined characteristics]. #moderate #high
    - ITEM-0357 IA-4(5) Dynamic Management
      Related controls: [ITEM-0091].
      - REQ-0542 Manage individual identifiers dynamically in accordance with [Assignment: organization-defined dynamic identifier policy].
    - ITEM-0358 IA-4(6) Cross-organization Management
      Related controls: [ITEM-0202], [ITEM-0341], [ITEM-0361].
      - REQ-0543 Coordinate with the following external organizations for cross-organization management of identifiers: [Assignment: organization-defined external organizations].
    - ITEM-0359 IA-4(8) Pairwise Pseudonymous Identifiers
      Related controls: [ITEM-0361].
      - REQ-0544 Generate pairwise pseudonymous identifiers.
    - ITEM-0360 IA-4(9) Attribute Maintenance and Protection
      - REQ-0545 Maintain the attributes for each uniquely identified individual, device, or service in [Assignment: organization-defined protected central storage].
  - ITEM-0361 IA-5 Authenticator Management
    Related controls: [ITEM-0016], [ITEM-0062], [ITEM-0256], [ITEM-0341], [ITEM-0354], [ITEM-0378], [ITEM-0379], [ITEM-0385], [ITEM-0451], [ITEM-0492], [ITEM-0545], [ITEM-0818], [ITEM-0823].
    - REQ-0546 (a) Verifying, as part of the initial authenticator distribution, the identity of the individual, group, role, service, or device receiving the authenticator; #low #moderate #high
    - REQ-0547 (b) Establishing initial authenticator content for any authenticators issued by the organization; #low #moderate #high
    - REQ-0548 (c) Ensuring that authenticators have sufficient strength of mechanism for their intended use; #low #moderate #high
    - REQ-0549 (d) Establishing and implementing administrative procedures for initial authenticator distribution, for lost or compromised or damaged authenticators, and for revoking authenticators; #low #moderate #high
    - REQ-0550 (e) Changing default authenticators prior to first use; #low #moderate #high
    - REQ-0551 (f) Changing or refreshing authenticators [Assignment: organization-defined time period by authenticator type] or when [Assignment: organization-defined events] occur; #low #moderate #high
    - REQ-0552 (g) Protecting authenticator content from unauthorized disclosure and modification; #low #moderate #high
    - REQ-0553 (h) Requiring individuals to take, and having devices implement, specific controls to protect authenticators; and #low #moderate #high
    - REQ-0554 (i) Changing authenticators for group or role accounts when membership to those accounts changes. #low #moderate #high
    - ITEM-0362 IA-5(1) Password-based Authentication
      Related controls: [ITEM-0377].
      - REQ-0555 (a) Maintain a list of commonly-used, expected, or compromised passwords and update the list [Assignment: organization-defined frequency] and when organizational passwords are suspected to have been compromised directly or indirectly; #low #moderate #high
      - REQ-0556 (b) Verify, when users create or update passwords, that the passwords are not found on the list of commonly-used, expected, or compromised passwords in IA-5(1)(a); #low #moderate #high
      - REQ-0557 (c) Transmit passwords only over cryptographically-protected channels; #low #moderate #high
      - REQ-0558 (d) Store passwords using an approved salted key derivation function, preferably using a keyed hash; #low #moderate #high
      - REQ-0559 (e) Require immediate selection of a new password upon account recovery; #low #moderate #high
      - REQ-0560 (f) Allow user selection of long passwords and passphrases, including spaces and all printable characters; #low #moderate #high
      - REQ-0561 (g) Employ automated tools to assist the user in selecting strong password authenticators; and #low #moderate #high
      - REQ-0562 (h) Enforce the following composition and complexity rules: [Assignment: organization-defined composition and complexity rules]. #low #moderate #high
    - ITEM-0363 IA-5(2) Public Key-based Authentication
      Related controls: [ITEM-0350], [ITEM-0832].
      - REQ-0563 (a) For public key-based authentication: #moderate #high
        (a)(1) Enforce authorized access to the corresponding private key; and
        (a)(2) Map the authenticated identity to the account of the individual or group; and
      - REQ-0564 (b) When public key infrastructure (PKI) is used: #moderate #high
        (b)(1) Validate certificates by constructing and verifying a certification path to an accepted trust anchor, including checking certificate status information; and
        (b)(2) Implement a local cache of revocation data to support path discovery and validation.
    - ITEM-0364 IA-5(5) Change Authenticators Prior to Delivery
      - REQ-0565 Require developers and installers of system components to provide unique authenticators or change default authenticators prior to delivery and installation.
    - ITEM-0365 IA-5(6) Protection of Authenticators
      Related controls: [ITEM-0634].
      - REQ-0566 Protect authenticators commensurate with the security category of the information to which use of the authenticator permits access. #moderate #high
    - ITEM-0366 IA-5(7) No Embedded Unencrypted Static Authenticators
      - REQ-0567 Ensure that unencrypted static authenticators are not embedded in applications or other forms of static storage.
    - ITEM-0367 IA-5(8) Multiple System Accounts
      Related controls: [ITEM-0604].
      - REQ-0568 Implement [Assignment: organization-defined security controls] to manage the risk of compromise due to individuals having accounts on multiple systems.
    - ITEM-0368 IA-5(9) Federated Credential Management
      Related controls: [ITEM-0172], [ITEM-0202].
      - REQ-0569 Use the following external organizations to federate credentials: [Assignment: organization-defined external organizations].
    - ITEM-0369 IA-5(10) Dynamic Credential Binding
      Related controls: [ITEM-0202], [ITEM-0361].
      - REQ-0570 Bind identities and authenticators dynamically using the following rules: [Assignment: organization-defined binding rules].
    - ITEM-0370 IA-5(12) Biometric Authentication Performance
      Related controls: [ITEM-0073].
      - REQ-0571 For biometric-based authentication, employ mechanisms that satisfy the following biometric quality requirements [Assignment: organization-defined biometric quality requirements].
    - ITEM-0371 IA-5(13) Expiration of Cached Authenticators
      - REQ-0572 Prohibit the use of cached authenticators after [Assignment: organization-defined time period].
    - ITEM-0372 IA-5(14) Managing Content of PKI Trust Stores
      - REQ-0573 For PKI-based authentication, employ an organization-wide methodology for managing the content of PKI trust stores installed across all platforms, including networks, operating systems, browsers, and applications.
    - ITEM-0373 IA-5(15) GSA-approved Products and Services
      - REQ-0574 Use only General Services Administration-approved products and services for identity, credential, and access management.
    - ITEM-0374 IA-5(16) In-person or Trusted External Party Authenticator Issuance
      Related controls: [ITEM-0388].
      - REQ-0575 Require that the issuance of [Assignment: organization-defined types of and/or specific authenticators] be conducted [Selection: in person; by a trusted external party] before [Assignment: organization-defined registration authority] with authorization by [Assignment: organization-defined personnel or roles].
    - ITEM-0375 IA-5(17) Presentation Attack Detection for Biometric Authenticators
      Related controls: [ITEM-0073].
      - REQ-0576 Employ presentation attack detection mechanisms for biometric-based authentication.
    - ITEM-0376 IA-5(18) Password Managers
      - REQ-0577 (a) Employ [Assignment: organization-defined password managers] to generate and manage passwords; and
      - REQ-0578 (b) Protect the passwords using [Assignment: organization-defined controls].
  - ITEM-0377 IA-6 Authentication Feedback
    Related controls: [ITEM-0016].
    - REQ-0579 Obscure feedback of authentication information during the authentication process to protect the information from possible exploitation and use by unauthorized individuals. #low #moderate #high
  - ITEM-0378 IA-7 Cryptographic Module Authentication
    Related controls: [ITEM-0016], [ITEM-0361], [ITEM-0662], [ITEM-0818], [ITEM-0823].
    - REQ-0580 Implement mechanisms for authentication to a cryptographic module that meet the requirements of applicable laws, executive orders, directives, policies, regulations, standards, and guidelines for such authentication. #low #moderate #high
  - ITEM-0379 IA-8 Identification and Authentication (Non-organizational Users)
    Related controls: [ITEM-0003], [ITEM-0062], [ITEM-0090], [ITEM-0102], [ITEM-0110], [ITEM-0163], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0386], [ITEM-0387], [ITEM-0395], [ITEM-0451], [ITEM-0636], [ITEM-0662], [ITEM-0809].
    - REQ-0581 Uniquely identify and authenticate non-organizational users or processes acting on behalf of non-organizational users. #low #moderate #high
    - ITEM-0380 IA-8(1) Acceptance of PIV Credentials from Other Agencies
      Related controls: [ITEM-0496].
      - REQ-0582 Accept and electronically verify Personal Identity Verification-compliant credentials from other federal agencies. #low #moderate #high
    - ITEM-0381 IA-8(2) Acceptance of External Authenticators
      - REQ-0583 (a) Accept only external authenticators that are NIST-compliant; and #low #moderate #high
      - REQ-0584 (b) Document and maintain a list of accepted external authenticators. #low #moderate #high
    - ITEM-0382 IA-8(4) Use of Defined Profiles
      - REQ-0585 Conform to the following profiles for identity management [Assignment: organization-defined identity management profiles]. #low #moderate #high
    - ITEM-0383 IA-8(5) Acceptance of PIV-I Credentials
      - REQ-0586 Accept and verify federated or PKI credentials that meet [Assignment: organization-defined policy].
    - ITEM-0384 IA-8(6) Disassociability
      - REQ-0587 Implement the following measures to disassociate user attributes or identifier assertion relationships among individuals, credential service providers, and relying parties: [Assignment: organization-defined measures].
  - ITEM-0385 IA-9 Service Identification and Authentication
    Related controls: [ITEM-0350], [ITEM-0354], [ITEM-0361], [ITEM-0395], [ITEM-0809].
    - REQ-0588 Uniquely identify and authenticate [Assignment: organization-defined system services and applications] before establishing communications with devices, users, or other services or applications.
  - ITEM-0386 IA-10 Adaptive Authentication
    Related controls: [ITEM-0341], [ITEM-0379].
    - REQ-0589 Require individuals accessing the system to employ [Assignment: organization-defined supplemental authentication techniques or mechanisms] under specific [Assignment: organization-defined circumstances or situations].
  - ITEM-0387 IA-11 Re-authentication
    Related controls: [ITEM-0016], [ITEM-0084], [ITEM-0341], [ITEM-0350], [ITEM-0354], [ITEM-0379].
    - REQ-0590 Require users to re-authenticate when [Assignment: organization-defined circumstances or situations]. #low #moderate #high
  - ITEM-0388 IA-12 Identity Proofing
    Related controls: [ITEM-0061], [ITEM-0340], [ITEM-0341], [ITEM-0350], [ITEM-0354], [ITEM-0361], [ITEM-0377], [ITEM-0379], [ITEM-0395].
    - REQ-0591 (a) Identity proof users that require accounts for logical access to systems based on appropriate identity assurance level requirements as specified in applicable standards and guidelines; #moderate #high
    - REQ-0592 (b) Resolve user identities to a unique individual; and #moderate #high
    - REQ-0593 (c) Collect, validate, and verify identity evidence. #moderate #high
    - ITEM-0389 IA-12(1) Supervisor Authorization
      - REQ-0594 Require that the registration process to receive an account for logical access includes supervisor or sponsor authorization.
    - ITEM-0390 IA-12(2) Identity Evidence
      - REQ-0595 Require evidence of individual identification be presented to the registration authority. #moderate #high
    - ITEM-0391 IA-12(3) Identity Evidence Validation and Verification
      - REQ-0596 Require that the presented identity evidence be validated and verified through [Assignment: organization-defined methods of validation and verification]. #moderate #high
    - ITEM-0392 IA-12(4) In-person Validation and Verification
      - REQ-0597 Require that the validation and verification of identity evidence be conducted in person before a designated registration authority. #high
    - ITEM-0393 IA-12(5) Address Confirmation
      Related controls: [ITEM-0388].
      - REQ-0598 Require that a [Selection: registration code; notice of proofing] be delivered through an out-of-band channel to verify the users address (physical or digital) of record. #moderate #high
    - ITEM-0394 IA-12(6) Accept Externally-proofed Identities
      Related controls: [ITEM-0350], [ITEM-0354], [ITEM-0361], [ITEM-0379].
      - REQ-0599 Accept externally-proofed identities at [Assignment: organization-defined identity assurance level].
  - ITEM-0395 IA-13 Identity Providers and Authorization Servers
    Related controls: [ITEM-0016], [ITEM-0341], [ITEM-0350], [ITEM-0379], [ITEM-0385], [ITEM-0388].
    - REQ-0600 Employ identity providers and authorization servers to manage user, device, and non-person entity (NPE) identities, attributes, and access rights supporting authentication and authorization decisions in accordance with [Assignment: organization-defined policy] using [Assignment: organization-defined mechanisms].
    - ITEM-0396 IA-13(1) Protection of Cryptographic Keys
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-0601 Cryptographic keys that protect access tokens are generated, managed, and protected from disclosure and misuse.
    - ITEM-0397 IA-13(2) Verification of Identity Assertions and Access Tokens
      - REQ-0602 The source and integrity of identity assertions and access tokens are verified before granting access to system and information resources.
    - ITEM-0398 IA-13(3) Token Management
      - REQ-0603 (a) generated;
      - REQ-0604 (b) issued;
      - REQ-0605 (c) refreshed;
      - REQ-0606 (d) revoked;
      - REQ-0607 (e) time-restricted; and
      - REQ-0608 (f) audience-restricted.
- ITEM-0399 Incident Response
  The incident response family (IR) of NIST SP 800-53 Revision 5.
  - ITEM-0400 IR-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0609 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] incident response policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the incident response policy and the associated incident response controls;
    - REQ-0610 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the incident response policy and procedures; and #low #moderate #high #privacy
    - REQ-0611 (c) Review and update the current incident response: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0401 IR-2 Incident Response Training
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0147], [ITEM-0299], [ITEM-0405], [ITEM-0409], [ITEM-0434], [ITEM-0436].
    - REQ-0612 (a) Provide incident response training to system users consistent with assigned roles and responsibilities: #low #moderate #high #privacy
      (a)(1) Within [Assignment: organization-defined time period] of assuming an incident response role or responsibility or acquiring system access;
      (a)(2) When required by system changes; and
      (a)(3) [Assignment: organization-defined frequency] thereafter; and
    - REQ-0613 (b) Review and update incident response training content [Assignment: organization-defined frequency] and following [Assignment: organization-defined events]. #low #moderate #high #privacy
    - ITEM-0402 IR-2(1) Simulated Events
      - REQ-0614 Incorporate simulated events into incident response training to facilitate the required response by personnel in crisis situations. #high
    - ITEM-0403 IR-2(2) Automated Training Environments
      - REQ-0615 Provide an incident response training environment using [Assignment: organization-defined automated mechanisms]. #high
    - ITEM-0404 IR-2(3) Breach
      - REQ-0616 Provide incident response training on how to identify and respond to a breach, including the organization’s process for reporting a breach. #privacy
  - ITEM-0405 IR-3 Incident Response Testing
    Related controls: [ITEM-0299], [ITEM-0302], [ITEM-0401], [ITEM-0409], [ITEM-0434], [ITEM-0570].
    - REQ-0617 Test the effectiveness of the incident response capability for the system [Assignment: organization-defined frequency] using the following tests: [Assignment: organization-defined tests]. #moderate #high #privacy
    - ITEM-0406 IR-3(1) Automated Testing
      - REQ-0618 Test the incident response capability using [Assignment: organization-defined automated mechanisms].
    - ITEM-0407 IR-3(2) Coordination with Related Plans
      - REQ-0619 Coordinate incident response testing with organizational elements responsible for related plans. #moderate #high
    - ITEM-0408 IR-3(3) Continuous Improvement
      - REQ-0620 (a) Determine the effectiveness of incident response processes;
      - REQ-0621 (b) Continuously improve incident response processes; and
      - REQ-0622 (c) Provide incident response measures and metrics that are accurate, consistent, and in a reproducible format.
  - ITEM-0409 IR-4 Incident Handling
    Related controls: [ITEM-0115], [ITEM-0163], [ITEM-0172], [ITEM-0256], [ITEM-0291], [ITEM-0299], [ITEM-0302], [ITEM-0401], [ITEM-0405], [ITEM-0425], [ITEM-0427], [ITEM-0434], [ITEM-0507], [ITEM-0544], [ITEM-0568], [ITEM-0675], [ITEM-0777], [ITEM-0782], [ITEM-0913], [ITEM-0918], [ITEM-0947].
    - REQ-0623 (a) Implement an incident handling capability for incidents that is consistent with the incident response plan and includes preparation, detection and analysis, containment, eradication, and recovery; #low #moderate #high #privacy
    - REQ-0624 (b) Coordinate incident handling activities with contingency planning activities; #low #moderate #high #privacy
    - REQ-0625 (c) Incorporate lessons learned from ongoing incident handling activities into incident response procedures, training, and testing, and implement the resulting changes accordingly; and #low #moderate #high #privacy
    - REQ-0626 (d) Ensure the rigor, intensity, scope, and results of incident handling activities are comparable and predictable across the organization. #low #moderate #high #privacy
    - ITEM-0410 IR-4(1) Automated Incident Handling Processes
      - REQ-0627 Support the incident handling process using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0411 IR-4(2) Dynamic Reconfiguration
      Related controls: [ITEM-0003], [ITEM-0030], [ITEM-0234].
      - REQ-0628 Include the following types of dynamic reconfiguration for [Assignment: organization-defined system components] as part of the incident response capability: [Assignment: organization-defined types of dynamic reconfiguration].
    - ITEM-0412 IR-4(3) Continuity of Operations
      - REQ-0629 Identify [Assignment: organization-defined classes of incidents] and take the following actions in response to those incidents to ensure continuation of organizational mission and business functions: [Assignment: organization-defined actions].
    - ITEM-0413 IR-4(4) Information Correlation
      - REQ-0630 Correlate incident information and individual incident responses to achieve an organization-wide perspective on incident awareness and response. #high
    - ITEM-0414 IR-4(5) Automatic Disabling of System
      - REQ-0631 Implement a configurable capability to automatically disable the system if [Assignment: organization-defined security violations] are detected.
    - ITEM-0415 IR-4(6) Insider Threats
      - REQ-0632 Implement an incident handling capability for incidents involving insider threats.
    - ITEM-0416 IR-4(7) Insider Threats — Intra-organization Coordination
      - REQ-0633 Coordinate an incident handling capability for insider threats that includes the following organizational entities [Assignment: organization-defined entities].
    - ITEM-0417 IR-4(8) Correlation with External Organizations
      Related controls: [ITEM-0202], [ITEM-0572].
      - REQ-0634 Coordinate with [Assignment: organization-defined external organizations] to correlate and share [Assignment: organization-defined incident information] to achieve a cross-organization perspective on incident awareness and more effective incident responses.
    - ITEM-0418 IR-4(9) Dynamic Response Capability
      - REQ-0635 Employ [Assignment: organization-defined dynamic response capabilities] to respond to incidents.
    - ITEM-0419 IR-4(10) Supply Chain Coordination
      Related controls: [ITEM-0212], [ITEM-0442], [ITEM-0709], [ITEM-1026].
      - REQ-0636 Coordinate incident handling activities involving supply chain events with other organizations involved in the supply chain.
    - ITEM-0420 IR-4(11) Integrated Incident Response Team
      Related controls: [ITEM-0142].
      - REQ-0637 Establish and maintain an integrated incident response team that can be deployed to any location identified by the organization in [Assignment: organization-defined time period]. #high
    - ITEM-0421 IR-4(12) Malicious Code and Forensic Analysis
      - REQ-0638 Analyze malicious code and/or other residual artifacts remaining in the system after the incident.
    - ITEM-0422 IR-4(13) Behavior Analysis
      - REQ-0639 Analyze anomalous or suspected adversarial behavior in or related to [Assignment: organization-defined environments or resources].
    - ITEM-0423 IR-4(14) Security Operations Center
      - REQ-0640 Establish and maintain a security operations center.
    - ITEM-0424 IR-4(15) Public Relations and Reputation Repair
      - REQ-0641 (a) Manage public relations associated with an incident; and
      - REQ-0642 (b) Employ measures to repair the reputation of the organization.
  - ITEM-0425 IR-5 Incident Monitoring
    Related controls: [ITEM-0163], [ITEM-0172], [ITEM-0409], [ITEM-0427], [ITEM-0434], [ITEM-0507], [ITEM-0559], [ITEM-0777], [ITEM-0782], [ITEM-0913], [ITEM-0918], [ITEM-0947].
    - REQ-0643 Track and document incidents. #low #moderate #high #privacy
    - ITEM-0426 IR-5(1) Automated Tracking, Data Collection, and Analysis
      - REQ-0644 Track incidents and collect and analyze incident information using [Assignment: organization-defined automated mechanisms]. #high
  - ITEM-0427 IR-6 Incident Reporting
    Related controls: [ITEM-0256], [ITEM-0291], [ITEM-0409], [ITEM-0425], [ITEM-0434], [ITEM-0436].
    - REQ-0645 (a) Require personnel to report suspected incidents to the organizational incident response capability within [Assignment: organization-defined time period] ; and #low #moderate #high #privacy
    - REQ-0646 (b) Report incident information to [Assignment: organization-defined authorities]. #low #moderate #high #privacy
    - ITEM-0428 IR-6(1) Automated Reporting
      Related controls: [ITEM-0431].
      - REQ-0647 Report incidents using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0429 IR-6(2) Vulnerabilities Related to Incidents
      - REQ-0648 Report system vulnerabilities associated with reported incidents to [Assignment: organization-defined personnel or roles].
    - ITEM-0430 IR-6(3) Supply Chain Coordination
      Related controls: [ITEM-1026].
      - REQ-0649 Provide incident information to the provider of the product or service and other organizations involved in the supply chain or supply chain governance for systems or system components related to the incident. #moderate #high
  - ITEM-0431 IR-7 Incident Response Assistance
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0409], [ITEM-0427], [ITEM-0434], [ITEM-0580], [ITEM-0584], [ITEM-0709], [ITEM-0988].
    - REQ-0650 Provide an incident response support resource, integral to the organizational incident response capability, that offers advice and assistance to users of the system for the handling and reporting of incidents. #low #moderate #high #privacy
    - ITEM-0432 IR-7(1) Automation Support for Availability of Information and Support
      - REQ-0651 Increase the availability of incident response information and support using [Assignment: organization-defined automated mechanisms]. #moderate #high
    - ITEM-0433 IR-7(2) Coordination with External Providers
      - REQ-0652 (a) Establish a direct, cooperative relationship between its incident response capability and external providers of system protection capability; and
      - REQ-0653 (b) Identify organizational incident response team members to the external providers.
  - ITEM-0434 IR-8 Incident Response Plan
    Related controls: [ITEM-0003], [ITEM-0291], [ITEM-0302], [ITEM-0409], [ITEM-0431], [ITEM-0436], [ITEM-0507], [ITEM-0544], [ITEM-0736], [ITEM-0972], [ITEM-1026].
    - REQ-0654 (a) Develop an incident response plan that: #low #moderate #high #privacy
      (a)(1) Provides the organization with a roadmap for implementing its incident response capability;
      (a)(2) Describes the structure and organization of the incident response capability;
      (a)(3) Provides a high-level approach for how the incident response capability fits into the overall organization;
      (a)(4) Meets the unique requirements of the organization, which relate to mission, size, structure, and functions;
      (a)(5) Defines reportable incidents;
      (a)(6) Provides metrics for measuring the incident response capability within the organization;
      (a)(7) Defines the resources and management support needed to effectively maintain and mature an incident response capability;
      (a)(8) Addresses the sharing of incident information;
      (a)(9) Is reviewed and approved by [Assignment: organization-defined personnel or roles] [Assignment: organization-defined frequency] ; and
      (a)(10) Explicitly designates responsibility for incident response to [Assignment: organization-defined entities, personnel, or roles].
    - REQ-0655 (b) Distribute copies of the incident response plan to [Assignment: organization-defined incident response personnel]; #low #moderate #high #privacy
    - REQ-0656 (c) Update the incident response plan to address system and organizational changes or problems encountered during plan implementation, execution, or testing; #low #moderate #high #privacy
    - REQ-0657 (d) Communicate incident response plan changes to [Assignment: organization-defined incident response personnel (identified by name and/or by role) and organizational elements] ; and #low #moderate #high #privacy
    - REQ-0658 (e) Protect the incident response plan from unauthorized disclosure and modification. #low #moderate #high #privacy
    - ITEM-0435 IR-8(1) Breaches
      Related controls: [ITEM-0611], [ITEM-0612], [ITEM-0615], [ITEM-0618], [ITEM-0622], [ITEM-0628].
      - REQ-0659 (a) A process to determine if notice to individuals or other organizations, including oversight organizations, is needed; #privacy
      - REQ-0660 (b) An assessment process to determine the extent of the harm, embarrassment, inconvenience, or unfairness to affected individuals and any mechanisms to mitigate such harms; and #privacy
      - REQ-0661 (c) Identification of applicable privacy requirements. #privacy
  - ITEM-0436 IR-9 Information Spillage Response
    Related controls: [ITEM-0291], [ITEM-0427], [ITEM-0584], [ITEM-0585], [ITEM-0612], [ITEM-0615], [ITEM-0628], [ITEM-0651].
    - REQ-0662 (a) Assigning [Assignment: organization-defined personnel or roles] with responsibility for responding to information spills;
    - REQ-0663 (b) Identifying the specific information involved in the system contamination;
    - REQ-0664 (c) Alerting [Assignment: organization-defined personnel or roles] of the information spill using a method of communication not associated with the spill;
    - REQ-0665 (d) Isolating the contaminated system or system component;
    - REQ-0666 (e) Eradicating the information from the contaminated system or component;
    - REQ-0667 (f) Identifying other systems or system components that may have been subsequently contaminated; and
    - REQ-0668 (g) Performing the following additional actions: [Assignment: organization-defined actions].
    - ITEM-0437 IR-9(2) Training
      Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0299], [ITEM-0401].
      - REQ-0669 Provide information spillage response training [Assignment: organization-defined frequency].
    - ITEM-0438 IR-9(3) Post-spill Operations
      - REQ-0670 Implement the following procedures to ensure that organizational personnel impacted by information spills can continue to carry out assigned tasks while contaminated systems are undergoing corrective actions: [Assignment: organization-defined procedures].
    - ITEM-0439 IR-9(4) Exposure to Unauthorized Personnel
      - REQ-0671 Employ the following controls for personnel exposed to information not within assigned access authorizations: [Assignment: organization-defined controls].
- ITEM-0440 Maintenance
  The maintenance family (MA) of NIST SP 800-53 Revision 5.
  - ITEM-0441 MA-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0672 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] maintenance policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the maintenance policy and the associated maintenance controls;
    - REQ-0673 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the maintenance policy and procedures; and #low #moderate #high
    - REQ-0674 (c) Review and update the current maintenance: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0442 MA-2 Controlled Maintenance
    Related controls: [ITEM-0234], [ITEM-0239], [ITEM-0248], [ITEM-0251], [ITEM-0269], [ITEM-0451], [ITEM-0477], [ITEM-0533], [ITEM-0906], [ITEM-1011], [ITEM-1015], [ITEM-1030].
    - REQ-0675 (a) Schedule, document, and review records of maintenance, repair, and replacement on system components in accordance with manufacturer or vendor specifications and/or organizational requirements; #low #moderate #high
    - REQ-0676 (b) Approve and monitor all maintenance activities, whether performed on site or remotely and whether the system or system components are serviced on site or removed to another location; #low #moderate #high
    - REQ-0677 (c) Require that [Assignment: organization-defined personnel or roles] explicitly approve the removal of the system or system components from organizational facilities for off-site maintenance, repair, or replacement; #low #moderate #high
    - REQ-0678 (d) Sanitize equipment to remove the following information from associated media prior to removal from organizational facilities for off-site maintenance, repair, or replacement: [Assignment: organization-defined information]; #low #moderate #high
    - REQ-0679 (e) Check all potentially impacted controls to verify that the controls are still functioning properly following maintenance, repair, or replacement actions; and #low #moderate #high
    - REQ-0680 (f) Include the following information in organizational maintenance records: [Assignment: organization-defined information]. #low #moderate #high
    - ITEM-0443 MA-2(2) Automated Maintenance Activities
      Related controls: [ITEM-0444].
      - REQ-0681 (a) Schedule, conduct, and document maintenance, repair, and replacement actions for the system using [Assignment: organization-defined automated mechanisms] ; and #high
      - REQ-0682 (b) Produce up-to date, accurate, and complete records of all maintenance, repair, and replacement actions requested, scheduled, in process, and completed. #high
  - ITEM-0444 MA-3 Maintenance Tools
    Related controls: [ITEM-0442], [ITEM-0533].
    - REQ-0683 (a) Approve, control, and monitor the use of system maintenance tools; and #moderate #high
    - REQ-0684 (b) Review previously approved system maintenance tools [Assignment: organization-defined frequency]. #moderate #high
    - ITEM-0445 MA-3(1) Inspect Tools
      Related controls: [ITEM-0947].
      - REQ-0685 Inspect the maintenance tools used by maintenance personnel for improper or unauthorized modifications. #moderate #high
    - ITEM-0446 MA-3(2) Inspect Media
      Related controls: [ITEM-0913].
      - REQ-0686 Check media containing diagnostic and test programs for malicious code before the media are used in the system. #moderate #high
    - ITEM-0447 MA-3(3) Prevent Unauthorized Removal
      Related controls: [ITEM-0477].
      - REQ-0687 (a) Verifying that there is no organizational information contained on the equipment; #moderate #high
      - REQ-0688 (b) Sanitizing or destroying the equipment; #moderate #high
      - REQ-0689 (c) Retaining the equipment within the facility; or #moderate #high
      - REQ-0690 (d) Obtaining an exemption from [Assignment: organization-defined personnel or roles] explicitly authorizing removal of the equipment from the facility. #moderate #high
    - ITEM-0448 MA-3(4) Restricted Tool Use
      Related controls: [ITEM-0016], [ITEM-0061], [ITEM-0062].
      - REQ-0691 Restrict the use of maintenance tools to authorized personnel only.
    - ITEM-0449 MA-3(5) Execution with Privilege
      Related controls: [ITEM-0016], [ITEM-0062].
      - REQ-0692 Monitor the use of maintenance tools that execute with increased privilege.
    - ITEM-0450 MA-3(6) Software Updates and Patches
      Related controls: [ITEM-0016], [ITEM-0062].
      - REQ-0693 Inspect maintenance tools to ensure the latest software updates and patches are installed.
  - ITEM-0451 MA-4 Nonlocal Maintenance
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0062], [ITEM-0102], [ITEM-0151], [ITEM-0152], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0379], [ITEM-0442], [ITEM-0458], [ITEM-0544], [ITEM-0782], [ITEM-0815].
    - REQ-0694 (a) Approve and monitor nonlocal maintenance and diagnostic activities; #low #moderate #high
    - REQ-0695 (b) Allow the use of nonlocal maintenance and diagnostic tools only as consistent with organizational policy and documented in the security plan for the system; #low #moderate #high
    - REQ-0696 (c) Employ strong authentication in the establishment of nonlocal maintenance and diagnostic sessions; #low #moderate #high
    - REQ-0697 (d) Maintain records for nonlocal maintenance and diagnostic activities; and #low #moderate #high
    - REQ-0698 (e) Terminate session and network connections when nonlocal maintenance is completed. #low #moderate #high
    - ITEM-0452 MA-4(1) Logging and Review
      Related controls: [ITEM-0163], [ITEM-0190].
      - REQ-0699 (a) Log [Assignment: organization-defined audit events] for nonlocal maintenance and diagnostic sessions; and
      - REQ-0700 (b) Review the audit records of the maintenance and diagnostic sessions to detect anomalous behavior.
    - ITEM-0453 MA-4(3) Comparable Security and Sanitization
      Related controls: [ITEM-0477], [ITEM-0913], [ITEM-0947].
      - REQ-0701 (a) Require that nonlocal maintenance and diagnostic services be performed from a system that implements a security capability comparable to the capability implemented on the system being serviced; or #high
      - REQ-0702 (b) Remove the component to be serviced from the system prior to nonlocal maintenance or diagnostic services; sanitize the component (for organizational information); and after the service is performed, inspect and sanitize the component (for potentially malicious software) before reconnecting the component to the system. #high
    - ITEM-0454 MA-4(4) Authentication and Separation of Maintenance Sessions
      - REQ-0703 (a) Employing [Assignment: organization-defined authenticators that are replay resistant] ; and
      - REQ-0704 (b) Separating the maintenance sessions from other network sessions with the system by either:
        (b)(1) Physically separated communications paths; or
        (b)(2) Logically separated communications paths.
    - ITEM-0455 MA-4(5) Approvals and Notifications
      - REQ-0705 (a) Require the approval of each nonlocal maintenance session by [Assignment: organization-defined personnel or roles] ; and
      - REQ-0706 (b) Notify the following personnel or roles of the date and time of planned nonlocal maintenance: [Assignment: organization-defined personnel and roles].
    - ITEM-0456 MA-4(6) Cryptographic Protection
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-0707 Implement the following cryptographic mechanisms to protect the integrity and confidentiality of nonlocal maintenance and diagnostic communications: [Assignment: organization-defined cryptographic mechanisms].
    - ITEM-0457 MA-4(7) Disconnect Verification
      Related controls: [ITEM-0086].
      - REQ-0708 Verify session and network connection termination after the completion of nonlocal maintenance and diagnostic sessions.
  - ITEM-0458 MA-5 Maintenance Personnel
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0061], [ITEM-0062], [ITEM-0341], [ITEM-0379], [ITEM-0451], [ITEM-0471], [ITEM-0492], [ITEM-0496], [ITEM-0607], [ITEM-0636].
    - REQ-0709 (a) Establish a process for maintenance personnel authorization and maintain a list of authorized maintenance organizations or personnel; #low #moderate #high
    - REQ-0710 (b) Verify that non-escorted personnel performing maintenance on the system possess the required access authorizations; and #low #moderate #high
    - REQ-0711 (c) Designate organizational personnel with required access authorizations and technical competence to supervise the maintenance activities of personnel who do not possess the required access authorizations. #low #moderate #high
    - ITEM-0459 MA-5(1) Individuals Without Appropriate Access
      Related controls: [ITEM-0477], [ITEM-0544].
      - REQ-0712 (a) Implement procedures for the use of maintenance personnel that lack appropriate security clearances or are not U.S. citizens, that include the following requirements: #high
        (a)(1) Maintenance personnel who do not have needed access authorizations, clearances, or formal access approvals are escorted and supervised during the performance of maintenance and diagnostic activities on the system by approved organizational personnel who are fully cleared, have appropriate access authorizations, and are technically qualified; and
        (a)(2) Prior to initiating maintenance or diagnostic activities by personnel who do not have needed access authorizations, clearances or formal access approvals, all volatile information storage components within the system are sanitized and all nonvolatile storage media are removed or physically disconnected from the system and secured; and
      - REQ-0713 (b) Develop and implement [Assignment: organization-defined alternate controls] in the event a system component cannot be sanitized, removed, or disconnected from the system. #high
    - ITEM-0460 MA-5(2) Security Clearances for Classified Systems
      Related controls: [ITEM-0595].
      - REQ-0714 Verify that personnel performing maintenance and diagnostic activities on a system processing, storing, or transmitting classified information possess security clearances and formal access approvals for at least the highest classification level and for compartments of information on the system.
    - ITEM-0461 MA-5(3) Citizenship Requirements for Classified Systems
      Related controls: [ITEM-0595].
      - REQ-0715 Verify that personnel performing maintenance and diagnostic activities on a system processing, storing, or transmitting classified information are U.S. citizens.
    - ITEM-0462 MA-5(4) Foreign Nationals
      Related controls: [ITEM-0595].
      - REQ-0716 (a) Foreign nationals with appropriate security clearances are used to conduct maintenance and diagnostic activities on classified systems only when the systems are jointly owned and operated by the United States and foreign allied governments, or owned and operated solely by foreign allied governments; and
      - REQ-0717 (b) Approvals, consents, and detailed operational conditions regarding the use of foreign nationals to conduct maintenance and diagnostic activities on classified systems are fully documented within Memoranda of Agreements.
    - ITEM-0463 MA-5(5) Non-system Maintenance
      - REQ-0718 Ensure that non-escorted personnel performing maintenance activities not directly associated with the system but in the physical proximity of the system, have required access authorizations.
  - ITEM-0464 MA-6 Timely Maintenance
    Related controls: [ITEM-0269], [ITEM-0291], [ITEM-0312], [ITEM-0651], [ITEM-0736], [ITEM-0976], [ITEM-1009], [ITEM-1011], [ITEM-1015].
    - REQ-0719 Obtain maintenance support and/or spare parts for [Assignment: organization-defined system components] within [Assignment: organization-defined time period] of failure. #moderate #high
    - ITEM-0465 MA-6(1) Preventive Maintenance
      - REQ-0720 Perform preventive maintenance on [Assignment: organization-defined system components] at [Assignment: organization-defined time intervals].
    - ITEM-0466 MA-6(2) Predictive Maintenance
      - REQ-0721 Perform predictive maintenance on [Assignment: organization-defined system components] at [Assignment: organization-defined time intervals].
    - ITEM-0467 MA-6(3) Automated Support for Predictive Maintenance
      - REQ-0722 Transfer predictive maintenance data to a maintenance management system using [Assignment: organization-defined automated mechanisms].
  - ITEM-0468 MA-7 Field Maintenance
    Related controls: [ITEM-0442], [ITEM-0451], [ITEM-0458].
    - REQ-0723 Restrict or prohibit field maintenance on [Assignment: organization-defined systems or system components] to [Assignment: organization-defined trusted maintenance facilities].
- ITEM-0469 Media Protection
  The media protection family (MP) of NIST SP 800-53 Revision 5.
  - ITEM-0470 MP-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0724 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] media protection policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the media protection policy and the associated media protection controls;
    - REQ-0725 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the media protection policy and procedures; and #low #moderate #high #privacy
    - REQ-0726 (c) Review and update the current media protection: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0471 MP-2 Media Access
    Related controls: [ITEM-0115], [ITEM-0175], [ITEM-0291], [ITEM-0324], [ITEM-0332], [ITEM-0458], [ITEM-0473], [ITEM-0477], [ITEM-0492], [ITEM-0496], [ITEM-0818], [ITEM-0823], [ITEM-0868], [ITEM-0972].
    - REQ-0727 Restrict access to [Assignment: organization-defined types of digital and/or non-digital media] to [Assignment: organization-defined personnel or roles]. #low #moderate #high
  - ITEM-0472 MP-3 Media Marking
    Related controls: [ITEM-0091], [ITEM-0324], [ITEM-0475], [ITEM-0540], [ITEM-0972].
    - REQ-0728 (a) Mark system media indicating the distribution limitations, handling caveats, and applicable security markings (if any) of the information; and #moderate #high
    - REQ-0729 (b) Exempt [Assignment: organization-defined types of media exempted from marking] from marking if the media remain within [Assignment: organization-defined controlled areas]. #moderate #high
  - ITEM-0473 MP-4 Media Storage
    Related controls: [ITEM-0115], [ITEM-0291], [ITEM-0308], [ITEM-0324], [ITEM-0332], [ITEM-0471], [ITEM-0483], [ITEM-0496], [ITEM-0544], [ITEM-0818], [ITEM-0823], [ITEM-0851], [ITEM-0868], [ITEM-0972].
    - REQ-0730 (a) Physically control and securely store [Assignment: organization-defined types of digital and/or non-digital media] within [Assignment: organization-defined controlled areas] ; and #moderate #high
    - REQ-0731 (b) Protect system media types defined in MP-4a until the media are destroyed or sanitized using approved equipment, techniques, and procedures. #moderate #high
    - ITEM-0474 MP-4(2) Automated Restricted Access
      Related controls: [ITEM-0016], [ITEM-0151], [ITEM-0163], [ITEM-0175], [ITEM-0190], [ITEM-0496].
      - REQ-0732 Restrict access to media storage areas and log access attempts and access granted using [Assignment: organization-defined automated mechanisms].
  - ITEM-0475 MP-5 Media Transport
    Related controls: [ITEM-0073], [ITEM-0115], [ITEM-0291], [ITEM-0324], [ITEM-0472], [ITEM-0473], [ITEM-0533], [ITEM-0544], [ITEM-0818], [ITEM-0823], [ITEM-0851], [ITEM-0868].
    - REQ-0733 (a) Protect and control [Assignment: organization-defined types of system media] during transport outside of controlled areas using [Assignment: organization-defined controls]; #moderate #high
    - REQ-0734 (b) Maintain accountability for system media during transport outside of controlled areas; #moderate #high
    - REQ-0735 (c) Document activities associated with the transport of system media; and #moderate #high
    - REQ-0736 (d) Restrict the activities associated with the transport of system media to authorized personnel. #moderate #high
    - ITEM-0476 MP-5(3) Custodians
      - REQ-0737 Employ an identified custodian during transport of system media outside of controlled areas.
  - ITEM-0477 MP-6 Media Sanitization
    Related controls: [ITEM-0016], [ITEM-0073], [ITEM-0188], [ITEM-0442], [ITEM-0444], [ITEM-0451], [ITEM-0458], [ITEM-0580], [ITEM-0972], [ITEM-0988], [ITEM-0994], [ITEM-1030].
    - REQ-0738 (a) Sanitize [Assignment: organization-defined system media] prior to disposal, release out of organizational control, or release for reuse using [Assignment: organization-defined sanitization techniques and procedures] ; and #low #moderate #high #privacy
    - REQ-0739 (b) Employ sanitization mechanisms with the strength and integrity commensurate with the security category or classification of the information. #low #moderate #high #privacy
    - ITEM-0478 MP-6(1) Review, Approve, Track, Document, and Verify
      - REQ-0740 Review, approve, track, document, and verify media sanitization and disposal actions. #high
    - ITEM-0479 MP-6(2) Equipment Testing
      - REQ-0741 Test sanitization equipment and procedures [Assignment: organization-defined frequency] to ensure that the intended sanitization is being achieved. #high
    - ITEM-0480 MP-6(3) Nondestructive Techniques
      - REQ-0742 Apply nondestructive sanitization techniques to portable storage devices prior to connecting such devices to the system under the following circumstances: [Assignment: organization-defined circumstances]. #high
    - ITEM-0481 MP-6(7) Dual Authorization
      Related controls: [ITEM-0016], [ITEM-0471].
      - REQ-0743 Enforce dual authorization for the sanitization of [Assignment: organization-defined system media].
    - ITEM-0482 MP-6(8) Remote Purging or Wiping of Information
      - REQ-0744 Provide the capability to purge or wipe information from [Assignment: organization-defined systems or system components] [Selection: remotely; under [Assignment: organization-defined conditions] ].
  - ITEM-0483 MP-7 Media Use
    Related controls: [ITEM-0115], [ITEM-0118], [ITEM-0545], [ITEM-0568], [ITEM-0868], [ITEM-0886].
    - REQ-0745 (a) [Selection: restrict; prohibit] the use of [Assignment: organization-defined types of system media] on [Assignment: organization-defined systems or system components] using [Assignment: organization-defined controls] ; and #low #moderate #high
    - REQ-0746 (b) Prohibit the use of portable storage devices in organizational systems when such devices have no identifiable owner. #low #moderate #high
    - ITEM-0484 MP-7(2) Prohibit Use of Sanitization-resistant Media
      Related controls: [ITEM-0477].
      - REQ-0747 Prohibit the use of sanitization-resistant media in organizational systems.
  - ITEM-0485 MP-8 Media Downgrading
    - REQ-0748 (a) Establish [Assignment: organization-defined system media downgrading process] that includes employing downgrading mechanisms with strength and integrity commensurate with the security category or classification of the information;
    - REQ-0749 (b) Verify that the system media downgrading process is commensurate with the security category and/or classification level of the information to be removed and the access authorizations of the potential recipients of the downgraded information;
    - REQ-0750 (c) Identify [Assignment: organization-defined system media requiring downgrading] ; and
    - REQ-0751 (d) Downgrade the identified system media using the established process.
    - ITEM-0486 MP-8(1) Documentation of Process
      - REQ-0752 Document system media downgrading actions.
    - ITEM-0487 MP-8(2) Equipment Testing
      - REQ-0753 Test downgrading equipment and procedures [Assignment: organization-defined frequency] to ensure that downgrading actions are being achieved.
    - ITEM-0488 MP-8(3) Controlled Unclassified Information
      - REQ-0754 Downgrade system media containing controlled unclassified information prior to public release.
    - ITEM-0489 MP-8(4) Classified Information
      - REQ-0755 Downgrade system media containing classified information prior to release to individuals without required access authorizations.
- ITEM-0490 Physical and Environmental Protection
  The physical and environmental protection family (PE) of NIST SP 800-53 Revision 5.
  - ITEM-0491 PE-1 Policy and Procedures
    Related controls: [ITEM-0142], [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0756 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] physical and environmental protection policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the physical and environmental protection policy and the associated physical and environmental protection controls;
    - REQ-0757 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the physical and environmental protection policy and procedures; and #low #moderate #high
    - REQ-0758 (c) Review and update the current physical and environmental protection: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0492 PE-2 Physical Access Authorizations
    Related controls: [ITEM-0142], [ITEM-0175], [ITEM-0354], [ITEM-0458], [ITEM-0471], [ITEM-0496], [ITEM-0504], [ITEM-0505], [ITEM-0512], [ITEM-0568], [ITEM-0595], [ITEM-0600], [ITEM-0603], [ITEM-0604].
    - REQ-0759 (a) Develop, approve, and maintain a list of individuals with authorized access to the facility where the system resides; #low #moderate #high
    - REQ-0760 (b) Issue authorization credentials for facility access; #low #moderate #high
    - REQ-0761 (c) Review the access list detailing authorized facility access by individuals [Assignment: organization-defined frequency] ; and #low #moderate #high
    - REQ-0762 (d) Remove individuals from the facility access list when access is no longer required. #low #moderate #high
    - ITEM-0493 PE-2(1) Access by Position or Role
      Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0062].
      - REQ-0763 Authorize physical access to the facility where the system resides based on position or role.
    - ITEM-0494 PE-2(2) Two Forms of Identification
      Related controls: [ITEM-0341], [ITEM-0354], [ITEM-0361].
      - REQ-0764 Require two forms of identification from the following forms of identification for visitor access to the facility where the system resides: [Assignment: organization-defined list of acceptable forms of identification].
    - ITEM-0495 PE-2(3) Restrict Unescorted Access
      Related controls: [ITEM-0594], [ITEM-0604].
      - REQ-0765 Restrict unescorted access to the facility where the system resides to personnel with [Selection (one or more): security clearances for all information contained within the system; formal access authorizations for all information contained within the system; need for access to all information contained within the system; [Assignment: organization-defined physical access authorizations] ].
  - ITEM-0496 PE-3 Physical Access Control
    Related controls: [ITEM-0142], [ITEM-0151], [ITEM-0163], [ITEM-0175], [ITEM-0195], [ITEM-0332], [ITEM-0350], [ITEM-0379], [ITEM-0458], [ITEM-0471], [ITEM-0473], [ITEM-0492], [ITEM-0504], [ITEM-0505], [ITEM-0512], [ITEM-0594], [ITEM-0595], [ITEM-0604], [ITEM-0607], [ITEM-0636], [ITEM-0851], [ITEM-0918], [ITEM-1011].
    - REQ-0766 (a) Enforce physical access authorizations at [Assignment: organization-defined entry and exit points] by: #low #moderate #high
      (a)(1) Verifying individual access authorizations before granting access to the facility; and
      (a)(2) Controlling ingress and egress to the facility using [Selection (one or more): [Assignment: organization-defined systems or devices] ; guards];
    - REQ-0767 (b) Maintain physical access audit logs for [Assignment: organization-defined entry or exit points]; #low #moderate #high
    - REQ-0768 (c) Control access to areas within the facility designated as publicly accessible by implementing the following controls: [Assignment: organization-defined physical access controls]; #low #moderate #high
    - REQ-0769 (d) Escort visitors and control visitor activity [Assignment: organization-defined circumstances]; #low #moderate #high
    - REQ-0770 (e) Secure keys, combinations, and other physical access devices; #low #moderate #high
    - REQ-0771 (f) Inventory [Assignment: organization-defined physical access devices] every [Assignment: organization-defined frequency] ; and #low #moderate #high
    - REQ-0772 (g) Change combinations and keys [Assignment: organization-defined frequency] and/or when keys are lost, combinations are compromised, or when individuals possessing the keys or combinations are transferred or terminated. #low #moderate #high
    - ITEM-0497 PE-3(1) System Access
      - REQ-0773 Enforce physical access authorizations to the system in addition to the physical access controls for the facility at [Assignment: organization-defined physical spaces]. #high
    - ITEM-0498 PE-3(2) Facility and Systems
      Related controls: [ITEM-0030], [ITEM-0782].
      - REQ-0774 Perform security checks [Assignment: organization-defined frequency] at the physical perimeter of the facility or system for exfiltration of information or removal of system components.
    - ITEM-0499 PE-3(3) Continuous Guards
      Related controls: [ITEM-0308], [ITEM-0312], [ITEM-0507].
      - REQ-0775 Employ guards to control [Assignment: organization-defined physical access points] to the facility where the system resides 24 hours per day, 7 days per week.
    - ITEM-0500 PE-3(4) Lockable Casings
      - REQ-0776 Use lockable physical casings to protect [Assignment: organization-defined system components] from unauthorized physical access.
    - ITEM-0501 PE-3(5) Tamper Protection
      Related controls: [ITEM-0748], [ITEM-1027], [ITEM-1030].
      - REQ-0777 Employ [Assignment: organization-defined anti-tamper technologies] to [Selection (one or more): detect; prevent] physical tampering or alteration of [Assignment: organization-defined hardware components] within the system.
    - ITEM-0502 PE-3(7) Physical Barriers
      - REQ-0778 Limit access using physical barriers.
    - ITEM-0503 PE-3(8) Access Control Vestibules
      - REQ-0779 Employ access control vestibules at [Assignment: organization-defined locations].
  - ITEM-0504 PE-4 Access Control for Transmission
    Related controls: [ITEM-0142], [ITEM-0354], [ITEM-0471], [ITEM-0473], [ITEM-0492], [ITEM-0496], [ITEM-0505], [ITEM-0515], [ITEM-0782], [ITEM-0809].
    - REQ-0780 Control physical access to [Assignment: organization-defined system distribution and transmission lines] within organizational facilities using [Assignment: organization-defined security controls]. #moderate #high
  - ITEM-0505 PE-5 Access Control for Output Devices
    Related controls: [ITEM-0492], [ITEM-0496], [ITEM-0504], [ITEM-0535].
    - REQ-0781 Control physical access to output from [Assignment: organization-defined output devices] to prevent unauthorized individuals from obtaining the output. #moderate #high
    - ITEM-0506 PE-5(2) Link to Individual Identity
      - REQ-0782 Link individual identity to receipt of output from output devices.
  - ITEM-0507 PE-6 Monitoring Physical Access
    Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0175], [ITEM-0190], [ITEM-0220], [ITEM-0332], [ITEM-0409], [ITEM-0434].
    - REQ-0783 (a) Monitor physical access to the facility where the system resides to detect and respond to physical security incidents; #low #moderate #high
    - REQ-0784 (b) Review physical access logs [Assignment: organization-defined frequency] and upon occurrence of [Assignment: organization-defined events] ; and #low #moderate #high
    - REQ-0785 (c) Coordinate results of reviews and investigations with the organizational incident response capability. #low #moderate #high
    - ITEM-0508 PE-6(1) Intrusion Alarms and Surveillance Equipment
      - REQ-0786 Monitor physical access to the facility where the system resides using physical intrusion alarms and surveillance equipment. #moderate #high
    - ITEM-0509 PE-6(2) Automated Intrusion Recognition and Responses
      Related controls: [ITEM-0918].
      - REQ-0787 Recognize [Assignment: organization-defined classes or types of intrusions] and initiate [Assignment: organization-defined response actions] using [Assignment: organization-defined automated mechanisms].
    - ITEM-0510 PE-6(3) Video Surveillance
      - REQ-0788 (a) Employ video surveillance of [Assignment: organization-defined operational areas];
      - REQ-0789 (b) Review video recordings [Assignment: organization-defined frequency] ; and
      - REQ-0790 (c) Retain video recordings for [Assignment: organization-defined time period].
    - ITEM-0511 PE-6(4) Monitoring Physical Access to Systems
      - REQ-0791 Monitor physical access to the system in addition to the physical access monitoring of the facility at [Assignment: organization-defined physical spaces]. #high
  - ITEM-0512 PE-8 Visitor Access Records
    Related controls: [ITEM-0492], [ITEM-0496], [ITEM-0507].
    - REQ-0792 (a) Maintain visitor access records to the facility where the system resides for [Assignment: organization-defined time period]; #low #moderate #high
    - REQ-0793 (b) Review visitor access records [Assignment: organization-defined frequency] ; and #low #moderate #high
    - REQ-0794 (c) Report anomalies in visitor access records to [Assignment: organization-defined personnel]. #low #moderate #high
    - ITEM-0513 PE-8(1) Automated Records Maintenance and Review
      - REQ-0795 Maintain and review visitor access records using [Assignment: organization-defined automated mechanisms]. #high
    - ITEM-0514 PE-8(3) Limit Personally Identifiable Information Elements
      Related controls: [ITEM-0636], [ITEM-0675].
      - REQ-0796 Limit personally identifiable information contained in visitor access records to the following elements identified in the privacy risk assessment: [Assignment: organization-defined elements]. #privacy
  - ITEM-0515 PE-9 Power Equipment and Cabling
    Related controls: [ITEM-0504].
    - REQ-0797 Protect power equipment and power cabling for the system from damage and destruction. #moderate #high
    - ITEM-0516 PE-9(1) Redundant Cabling
      - REQ-0798 Employ redundant power cabling paths that are physically separated by [Assignment: organization-defined distance].
    - ITEM-0517 PE-9(2) Automatic Voltage Controls
      - REQ-0799 Employ automatic voltage controls for [Assignment: organization-defined critical system components].
  - ITEM-0518 PE-10 Emergency Shutoff
    Related controls: [ITEM-0531].
    - REQ-0800 (a) Provide the capability of shutting off power to [Assignment: organization-defined system or individual system components] in emergency situations; #moderate #high
    - REQ-0801 (b) Place emergency shutoff switches or devices in [Assignment: organization-defined location] to facilitate access for authorized personnel; and #moderate #high
    - REQ-0802 (c) Protect emergency power shutoff capability from unauthorized activation. #moderate #high
  - ITEM-0519 PE-11 Emergency Power
    Related controls: [ITEM-0142], [ITEM-0291], [ITEM-0312].
    - REQ-0803 Provide an uninterruptible power supply to facilitate [Selection: an orderly shutdown of the system; transition of the system to long-term alternate power] in the event of a primary power source loss. #moderate #high
    - ITEM-0520 PE-11(1) Alternate Power Supply — Minimal Operational Capability
      - REQ-0804 Provide an alternate power supply for the system that is activated [Selection: manually; automatically] and that can maintain minimally required operational capability in the event of an extended loss of the primary power source. #high
    - ITEM-0521 PE-11(2) Alternate Power Supply — Self-contained
      - REQ-0805 (a) Self-contained;
      - REQ-0806 (b) Not reliant on external power generation; and
      - REQ-0807 (c) Capable of maintaining [Selection: minimally required operational capability; full operational capability] in the event of an extended loss of the primary power source.
  - ITEM-0522 PE-12 Emergency Lighting
    Related controls: [ITEM-0291], [ITEM-0312].
    - REQ-0808 Employ and maintain automatic emergency lighting for the system that activates in the event of a power outage or disruption and that covers emergency exits and evacuation routes within the facility. #low #moderate #high
    - ITEM-0523 PE-12(1) Essential Mission and Business Functions
      - REQ-0809 Provide emergency lighting for all areas within the facility supporting essential mission and business functions.
  - ITEM-0524 PE-13 Fire Protection
    Related controls: [ITEM-0142].
    - REQ-0810 Employ and maintain fire detection and suppression systems that are supported by an independent energy source. #low #moderate #high
    - ITEM-0525 PE-13(1) Detection Systems — Automatic Activation and Notification
      - REQ-0811 Employ fire detection systems that activate automatically and notify [Assignment: organization-defined personnel or roles] and [Assignment: organization-defined emergency responders] in the event of a fire. #moderate #high
    - ITEM-0526 PE-13(2) Suppression Systems — Automatic Activation and Notification
      - REQ-0812 (a) Employ fire suppression systems that activate automatically and notify [Assignment: organization-defined personnel or roles] and [Assignment: organization-defined emergency responders] ; and #high
      - REQ-0813 (b) Employ an automatic fire suppression capability when the facility is not staffed on a continuous basis. #high
    - ITEM-0527 PE-13(4) Inspections
      - REQ-0814 Ensure that the facility undergoes [Assignment: organization-defined frequency] fire protection inspections by authorized and qualified inspectors and identified deficiencies are resolved within [Assignment: organization-defined time period].
  - ITEM-0528 PE-14 Environmental Controls
    Related controls: [ITEM-0142], [ITEM-0291].
    - REQ-0815 (a) Maintain [Selection (one or more): temperature; humidity; pressure; radiation; [Assignment: organization-defined environmental control] ] levels within the facility where the system resides at [Assignment: organization-defined acceptable levels] ; and #low #moderate #high
    - REQ-0816 (b) Monitor environmental control levels [Assignment: organization-defined frequency]. #low #moderate #high
    - ITEM-0529 PE-14(1) Automatic Controls
      - REQ-0817 Employ the following automatic environmental controls in the facility to prevent fluctuations potentially harmful to the system: [Assignment: organization-defined automatic environmental controls].
    - ITEM-0530 PE-14(2) Monitoring with Alarms and Notifications
      - REQ-0818 Employ environmental control monitoring that provides an alarm or notification of changes potentially harmful to personnel or equipment to [Assignment: organization-defined personnel or roles].
  - ITEM-0531 PE-15 Water Damage Protection
    Related controls: [ITEM-0142], [ITEM-0518].
    - REQ-0819 Protect the system from damage resulting from water leakage by providing master shutoff or isolation valves that are accessible, working properly, and known to key personnel. #low #moderate #high
    - ITEM-0532 PE-15(1) Automation Support
      - REQ-0820 Detect the presence of water near the system and alert [Assignment: organization-defined personnel or roles] using [Assignment: organization-defined automated mechanisms]. #high
  - ITEM-0533 PE-16 Delivery and Removal
    Related controls: [ITEM-0239], [ITEM-0269], [ITEM-0442], [ITEM-0444], [ITEM-0475], [ITEM-0538], [ITEM-1009], [ITEM-1011], [ITEM-1015], [ITEM-1023].
    - REQ-0821 (a) Authorize and control [Assignment: organization-defined types of system components] entering and exiting the facility; and #low #moderate #high
    - REQ-0822 (b) Maintain records of the system components. #low #moderate #high
  - ITEM-0534 PE-17 Alternate Work Site
    Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0312].
    - REQ-0823 (a) Determine and document the [Assignment: organization-defined alternate work sites] allowed for use by employees; #moderate #high
    - REQ-0824 (b) Employ the following controls at alternate work sites: [Assignment: organization-defined controls]; #moderate #high
    - REQ-0825 (c) Assess the effectiveness of controls at alternate work sites; and #moderate #high
    - REQ-0826 (d) Provide a means for employees to communicate with information security and privacy personnel in case of incidents. #moderate #high
  - ITEM-0535 PE-18 Location of System Components
    Related controls: [ITEM-0291], [ITEM-0505], [ITEM-0536], [ITEM-0538], [ITEM-0636].
    - REQ-0827 Position system components within the facility to minimize potential damage from [Assignment: organization-defined physical and environmental hazards] and to minimize the opportunity for unauthorized access. #high
  - ITEM-0536 PE-19 Information Leakage
    Related controls: [ITEM-0110], [ITEM-0535], [ITEM-0538].
    - REQ-0828 Protect the system from information leakage due to electromagnetic signals emanations.
    - ITEM-0537 PE-19(1) National Emissions Policies and Procedures
      - REQ-0829 Protect system components, associated data communications, and networks in accordance with national Emissions Security policies and procedures based on the security category or classification of the information.
  - ITEM-0538 PE-20 Asset Monitoring and Tracking
    Related controls: [ITEM-0269], [ITEM-0533], [ITEM-0564].
    - REQ-0830 Employ [Assignment: organization-defined asset location technologies] to track and monitor the location and movement of [Assignment: organization-defined assets] within [Assignment: organization-defined controlled areas].
  - ITEM-0539 PE-21 Electromagnetic Pulse Protection
    Related controls: [ITEM-0535], [ITEM-0536].
    - REQ-0831 Employ [Assignment: organization-defined protective measures] against electromagnetic pulse damage for [Assignment: organization-defined system and system components].
  - ITEM-0540 PE-22 Component Marking
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0091], [ITEM-0472].
    - REQ-0832 Mark [Assignment: organization-defined system hardware components] indicating the impact level or classification level of the information permitted to be processed, stored, or transmitted by the hardware component.
  - ITEM-0541 PE-23 Facility Location
    Related controls: [ITEM-0291], [ITEM-0535], [ITEM-0536], [ITEM-0564], [ITEM-0565], [ITEM-0636].
    - REQ-0833 (a) Plan the location or site of the facility where the system resides considering physical and environmental hazards; and
    - REQ-0834 (b) For existing facilities, consider the physical and environmental hazards in the organizational risk management strategy.
- ITEM-0542 Planning
  The planning family (PL) of NIST SP 800-53 Revision 5.
  - ITEM-0543 PL-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0835 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] planning policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the planning policy and the associated planning controls;
    - REQ-0836 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the planning policy and procedures; and #low #moderate #high #privacy
    - REQ-0837 (c) Review and update the current planning: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0544 PL-2 System Security and Privacy Plans
    Related controls: [ITEM-0003], [ITEM-0062], [ITEM-0090], [ITEM-0102], [ITEM-0118], [ITEM-0208], [ITEM-0212], [ITEM-0220], [ITEM-0278], [ITEM-0287], [ITEM-0291], [ITEM-0302], [ITEM-0409], [ITEM-0434], [ITEM-0451], [ITEM-0458], [ITEM-0473], [ITEM-0475], [ITEM-0547], [ITEM-0548], [ITEM-0552], [ITEM-0553], [ITEM-0555], [ITEM-0562], [ITEM-0564], [ITEM-0565], [ITEM-0566], [ITEM-0567], [ITEM-0636], [ITEM-0652], [ITEM-0653], [ITEM-0674], [ITEM-0749], [ITEM-0761], [ITEM-0972], [ITEM-1009], [ITEM-1015].
    - REQ-0838 (a) Develop security and privacy plans for the system that: #low #moderate #high #privacy
      (a)(1) Are consistent with the organization’s enterprise architecture;
      (a)(2) Explicitly define the constituent system components;
      (a)(3) Describe the operational context of the system in terms of mission and business processes;
      (a)(4) Identify the individuals that fulfill system roles and responsibilities;
      (a)(5) Identify the information types processed, stored, and transmitted by the system;
      (a)(6) Provide the security categorization of the system, including supporting rationale;
      (a)(7) Describe any specific threats to the system that are of concern to the organization;
      (a)(8) Provide the results of a privacy risk assessment for systems processing personally identifiable information;
      (a)(9) Describe the operational environment for the system and any dependencies on or connections to other systems or system components;
      (a)(10) Provide an overview of the security and privacy requirements for the system;
      (a)(11) Identify any relevant control baselines or overlays, if applicable;
      (a)(12) Describe the controls in place or planned for meeting the security and privacy requirements, including a rationale for any tailoring decisions;
      (a)(13) Include risk determinations for security and privacy architecture and design decisions;
      (a)(14) Include security- and privacy-related activities affecting the system that require planning and coordination with [Assignment: organization-defined individuals or groups] ; and
      (a)(15) Are reviewed and approved by the authorizing official or designated representative prior to plan implementation.
    - REQ-0839 (b) Distribute copies of the plans and communicate subsequent changes to the plans to [Assignment: organization-defined personnel or roles]; #low #moderate #high #privacy
    - REQ-0840 (c) Review the plans [Assignment: organization-defined frequency]; #low #moderate #high #privacy
    - REQ-0841 (d) Update the plans to address changes to the system and environment of operation or problems identified during plan implementation or control assessments; and #low #moderate #high #privacy
    - REQ-0842 (e) Protect the plans from unauthorized disclosure and modification. #low #moderate #high #privacy
  - ITEM-0545 PL-4 Rules of Behavior
    Related controls: [ITEM-0003], [ITEM-0062], [ITEM-0077], [ITEM-0078], [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0118], [ITEM-0135], [ITEM-0142], [ITEM-0282], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0483], [ITEM-0604], [ITEM-0608], [ITEM-0674], [ITEM-0972].
    - REQ-0843 (a) Establish and provide to individuals requiring access to the system, the rules that describe their responsibilities and expected behavior for information and system usage, security, and privacy; #low #moderate #high #privacy
    - REQ-0844 (b) Receive a documented acknowledgment from such individuals, indicating that they have read, understand, and agree to abide by the rules of behavior, before authorizing access to information and the system; #low #moderate #high #privacy
    - REQ-0845 (c) Review and update the rules of behavior [Assignment: organization-defined frequency] ; and #low #moderate #high #privacy
    - REQ-0846 (d) Require individuals who have acknowledged a previous version of the rules of behavior to read and re-acknowledge [Selection (one or more): [Assignment: organization-defined frequency] ; when the rules are revised or updated]. #low #moderate #high #privacy
    - ITEM-0546 PL-4(1) Social Media and External Site/Application Usage Restrictions
      Related controls: [ITEM-0127], [ITEM-0195].
      - REQ-0847 (a) Use of social media, social networking sites, and external sites/applications; #low #moderate #high #privacy
      - REQ-0848 (b) Posting organizational information on public websites; and #low #moderate #high #privacy
      - REQ-0849 (c) Use of organization-provided identifiers (e.g., email addresses) and authentication secrets (e.g., passwords) for creating accounts on external sites/applications. #low #moderate #high #privacy
  - ITEM-0547 PL-7 Concept of Operations
    Related controls: [ITEM-0544], [ITEM-0657], [ITEM-0972].
    - REQ-0850 (a) Develop a Concept of Operations (CONOPS) for the system describing how the organization intends to operate the system from the perspective of information security and privacy; and
    - REQ-0851 (b) Review and update the CONOPS [Assignment: organization-defined frequency].
  - ITEM-0548 PL-8 Security and Privacy Architectures
    Related controls: [ITEM-0234], [ITEM-0256], [ITEM-0544], [ITEM-0547], [ITEM-0551], [ITEM-0559], [ITEM-0562], [ITEM-0653], [ITEM-0658], [ITEM-0674], [ITEM-0675], [ITEM-0749], [ITEM-0782].
    - REQ-0852 (a) Develop security and privacy architectures for the system that: #moderate #high #privacy
      (a)(1) Describe the requirements and approach to be taken for protecting the confidentiality, integrity, and availability of organizational information;
      (a)(2) Describe the requirements and approach to be taken for processing personally identifiable information to minimize privacy risk to individuals;
      (a)(3) Describe how the architectures are integrated into and support the enterprise architecture; and
      (a)(4) Describe any assumptions about, and dependencies on, external systems and services;
    - REQ-0853 (b) Review and update the architectures [Assignment: organization-defined frequency] to reflect changes in the enterprise architecture; and #moderate #high #privacy
    - REQ-0854 (c) Reflect planned architecture changes in security and privacy plans, Concept of Operations (CONOPS), criticality analysis, organizational procedures, and procurements and acquisitions. #moderate #high #privacy
    - ITEM-0549 PL-8(1) Defense in Depth
      Related controls: [ITEM-0766], [ITEM-0769], [ITEM-0855], [ITEM-0872].
      - REQ-0855 (a) Allocates [Assignment: organization-defined controls] to [Assignment: organization-defined locations and architectural layers] ; and
      - REQ-0856 (b) Ensures that the allocated controls operate in a coordinated and mutually reinforcing manner.
    - ITEM-0550 PL-8(2) Supplier Diversity
      Related controls: [ITEM-0855], [ITEM-1011].
      - REQ-0857 Require that [Assignment: organization-defined controls] allocated to [Assignment: organization-defined locations and architectural layers] are obtained from different suppliers.
  - ITEM-0551 PL-9 Central Management
    Related controls: [ITEM-0548], [ITEM-0565].
    - REQ-0858 Centrally manage [Assignment: organization-defined controls and related processes]. #privacy
  - ITEM-0552 PL-10 Baseline Selection
    Related controls: [ITEM-0544], [ITEM-0553], [ITEM-0634], [ITEM-0636], [ITEM-0675].
    - REQ-0859 Select a control baseline for the system. #low #moderate #high
  - ITEM-0553 PL-11 Baseline Tailoring
    Related controls: [ITEM-0552], [ITEM-0634], [ITEM-0636], [ITEM-0653], [ITEM-0675].
    - REQ-0860 Tailor the selected control baseline by applying specified tailoring actions. #low #moderate #high
- ITEM-0554 Program Management
  The program management family (PM) of NIST SP 800-53 Revision 5.
  - ITEM-0555 PM-1 Information Security Program Plan
    Related controls: [ITEM-0544], [ITEM-0575], [ITEM-0588], [ITEM-0653], [ITEM-0972], [ITEM-1009].
    - REQ-0861 (a) Develop and disseminate an organization-wide information security program plan that:
      (a)(1) Provides an overview of the requirements for the security program and a description of the security program management controls and common controls in place or planned for meeting those requirements;
      (a)(2) Includes the identification and assignment of roles, responsibilities, management commitment, coordination among organizational entities, and compliance;
      (a)(3) Reflects the coordination among organizational entities responsible for information security; and
      (a)(4) Is approved by a senior official with responsibility and accountability for the risk being incurred to organizational operations (including mission, functions, image, and reputation), organizational assets, individuals, other organizations, and the Nation;
    - REQ-0862 (b) Review and update the organization-wide information security program plan [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
    - REQ-0863 (c) Protect the information security program plan from unauthorized disclosure and modification.
  - ITEM-0556 PM-2 Information Security Program Leadership Role
    - REQ-0864 Appoint a senior agency information security officer with the mission and resources to coordinate, develop, implement, and maintain an organization-wide information security program.
  - ITEM-0557 PM-3 Information Security and Privacy Resources
    Related controls: [ITEM-0558], [ITEM-0657].
    - REQ-0865 (a) Include the resources needed to implement the information security and privacy programs in capital planning and investment requests and document all exceptions to this requirement; #privacy
    - REQ-0866 (b) Prepare documentation required for addressing information security and privacy programs in capital planning and investment requests in accordance with applicable laws, executive orders, directives, policies, regulations, standards; and #privacy
    - REQ-0867 (c) Make available for expenditure, the planned information security and privacy resources. #privacy
  - ITEM-0558 PM-4 Plan of Action and Milestones Process
    Related controls: [ITEM-0215], [ITEM-0220], [ITEM-0557], [ITEM-0651], [ITEM-0972].
    - REQ-0868 (a) Implement a process to ensure that plans of action and milestones for the information security, privacy, and supply chain risk management programs and associated organizational systems: #privacy
      (a)(1) Are developed and maintained;
      (a)(2) Document the remedial information security, privacy, and supply chain risk management actions to adequately respond to risk to organizational operations and assets, individuals, other organizations, and the Nation; and
      (a)(3) Are reported in accordance with established reporting requirements.
    - REQ-0869 (b) Review plans of action and milestones for consistency with the organizational risk management strategy and organization-wide priorities for risk response actions. #privacy
  - ITEM-0559 PM-5 System Inventory
    - REQ-0870 Develop and update [Assignment: organization-defined frequency] an inventory of organizational systems.
    - ITEM-0560 PM-5(1) Inventory of Personally Identifiable Information
      Related controls: [ITEM-0016], [ITEM-0269], [ITEM-0285], [ITEM-0287], [ITEM-0548], [ITEM-0580], [ITEM-0615], [ITEM-0622], [ITEM-0972], [ITEM-0988].
      - REQ-0871 Establish, maintain, and update [Assignment: organization-defined frequency] an inventory of all systems, applications, and projects that process personally identifiable information. #privacy
  - ITEM-0561 PM-6 Measures of Performance
    Related controls: [ITEM-0220], [ITEM-0565].
    - REQ-0872 Develop, monitor, and report on the results of information security and privacy measures of performance. #privacy
  - ITEM-0562 PM-7 Enterprise Architecture
    Related controls: [ITEM-0163], [ITEM-0544], [ITEM-0548], [ITEM-0567], [ITEM-0634], [ITEM-0658], [ITEM-0675], [ITEM-0749].
    - REQ-0873 Develop and maintain an enterprise architecture with consideration for information security, privacy, and the resulting risk to organizational operations and assets, individuals, other organizations, and the Nation. #privacy
    - ITEM-0563 PM-7(1) Offloading
      Related controls: [ITEM-0675].
      - REQ-0874 Offload [Assignment: organization-defined non-essential functions or services] to other systems, system components, or an external provider.
  - ITEM-0564 PM-8 Critical Infrastructure Plan
    Related controls: [ITEM-0291], [ITEM-0302], [ITEM-0535], [ITEM-0544], [ITEM-0565], [ITEM-0567], [ITEM-0575], [ITEM-0636], [ITEM-0972].
    - REQ-0875 Address information security and privacy issues in the development, documentation, and updating of a critical infrastructure and key resources protection plan. #privacy
  - ITEM-0565 PM-9 Risk Management Strategy
    Related controls: [ITEM-0002], [ITEM-0150], [ITEM-0134], [ITEM-0207], [ITEM-0208], [ITEM-0215], [ITEM-0217], [ITEM-0220], [ITEM-0233], [ITEM-0290], [ITEM-0340], [ITEM-0400], [ITEM-0441], [ITEM-0470], [ITEM-0491], [ITEM-0543], [ITEM-0544], [ITEM-0556], [ITEM-0564], [ITEM-0575], [ITEM-0586], [ITEM-0588], [ITEM-0593], [ITEM-0611], [ITEM-0612], [ITEM-0615], [ITEM-0633], [ITEM-0636], [ITEM-0653], [ITEM-0656], [ITEM-0662], [ITEM-0765], [ITEM-0877], [ITEM-0905], [ITEM-0972], [ITEM-1008], [ITEM-1009].
    - REQ-0876 (a) Develops a comprehensive strategy to manage: #privacy
      (a)(1) Security risk to organizational operations and assets, individuals, other organizations, and the Nation associated with the operation and use of organizational systems; and
      (a)(2) Privacy risk to individuals resulting from the authorized processing of personally identifiable information;
    - REQ-0877 (b) Implement the risk management strategy consistently across the organization; and #privacy
    - REQ-0878 (c) Review and update the risk management strategy [Assignment: organization-defined frequency] or as required, to address organizational changes. #privacy
  - ITEM-0566 PM-10 Authorization Process
    Related controls: [ITEM-0217], [ITEM-0220], [ITEM-0544].
    - REQ-0879 (a) Manage the security and privacy state of organizational systems and the environments in which those systems operate through authorization processes; #privacy
    - REQ-0880 (b) Designate individuals to fulfill specific roles and responsibilities within the organizational risk management process; and #privacy
    - REQ-0881 (c) Integrate the authorization processes into an organization-wide risk management program. #privacy
  - ITEM-0567 PM-11 Mission and Business Process Definition
    Related controls: [ITEM-0291], [ITEM-0544], [ITEM-0562], [ITEM-0564], [ITEM-0634], [ITEM-0636], [ITEM-0653], [ITEM-0657].
    - REQ-0882 (a) Define organizational mission and business processes with consideration for information security and privacy and the resulting risk to organizational operations, organizational assets, individuals, other organizations, and the Nation; and #privacy
    - REQ-0883 (b) Determine information protection and personally identifiable information processing needs arising from the defined mission and business processes; and #privacy
    - REQ-0884 (c) Review and revise the mission and business processes [Assignment: organization-defined frequency]. #privacy
  - ITEM-0568 PM-12 Insider Threat Program
    Related controls: [ITEM-0062], [ITEM-0135], [ITEM-0163], [ITEM-0172], [ITEM-0183], [ITEM-0190], [ITEM-0195], [ITEM-0220], [ITEM-0354], [ITEM-0409], [ITEM-0483], [ITEM-0492], [ITEM-0572], [ITEM-0595], [ITEM-0600], [ITEM-0603], [ITEM-0607], [ITEM-0608], [ITEM-0782], [ITEM-0877], [ITEM-0918], [ITEM-0570].
    - REQ-0885 Implement an insider threat program that includes a cross-discipline insider threat incident handling team.
  - ITEM-0569 PM-13 Security and Privacy Workforce
    Related controls: [ITEM-0135], [ITEM-0142].
    - REQ-0886 Establish a security and privacy workforce development and improvement program. #privacy
  - ITEM-0570 PM-14 Testing, Training, and Monitoring
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0220], [ITEM-0302], [ITEM-0405], [ITEM-0568], [ITEM-0918].
    - REQ-0887 (a) Implement a process for ensuring that organizational plans for conducting security and privacy testing, training, and monitoring activities associated with organizational systems: #privacy
      (a)(1) Are developed and maintained; and
      (a)(2) Continue to be executed; and
    - REQ-0888 (b) Review testing, training, and monitoring plans for consistency with the organizational risk management strategy and organization-wide priorities for risk response actions. #privacy
  - ITEM-0571 PM-15 Security and Privacy Groups and Associations
    Related controls: [ITEM-0726], [ITEM-0942].
    - REQ-0889 (a) To facilitate ongoing security and privacy education and training for organizational personnel;
    - REQ-0890 (b) To maintain currency with recommended security and privacy practices, techniques, and technologies; and
    - REQ-0891 (c) To share current security and privacy information, including threats, vulnerabilities, and incidents.
  - ITEM-0572 PM-16 Threat Awareness Program
    Related controls: [ITEM-0409], [ITEM-0568].
    - REQ-0892 Implement a threat awareness program that includes a cross-organization information-sharing capability for threat intelligence.
    - ITEM-0573 PM-16(1) Automated Means for Sharing Threat Intelligence
      - REQ-0893 Employ automated mechanisms to maximize the effectiveness of sharing threat intelligence information.
  - ITEM-0574 PM-17 Protecting Controlled Unclassified Information on External Systems
    Related controls: [ITEM-0217], [ITEM-0566].
    - REQ-0894 (a) Establish policy and procedures to ensure that requirements for the protection of controlled unclassified information that is processed, stored or transmitted on external systems, are implemented in accordance with applicable laws, executive orders, directives, policies, regulations, and standards; and #privacy
    - REQ-0895 (b) Review and update the policy and procedures [Assignment: organization-defined frequency]. #privacy
  - ITEM-0575 PM-18 Privacy Program Plan
    Related controls: [ITEM-0564], [ITEM-0565], [ITEM-0576].
    - REQ-0896 (a) Develop and disseminate an organization-wide privacy program plan that provides an overview of the agency’s privacy program, and: #privacy
      (a)(1) Includes a description of the structure of the privacy program and the resources dedicated to the privacy program;
      (a)(2) Provides an overview of the requirements for the privacy program and a description of the privacy program management controls and common controls in place or planned for meeting those requirements;
      (a)(3) Includes the role of the senior agency official for privacy and the identification and assignment of roles of other privacy officials and staff and their responsibilities;
      (a)(4) Describes management commitment, compliance, and the strategic goals and objectives of the privacy program;
      (a)(5) Reflects coordination among organizational entities responsible for the different aspects of privacy; and
      (a)(6) Is approved by a senior official with responsibility and accountability for the privacy risk being incurred to organizational operations (including mission, functions, image, and reputation), organizational assets, individuals, other organizations, and the Nation; and
    - REQ-0897 (b) Update the plan [Assignment: organization-defined frequency] and to address changes in federal privacy laws and policy and organizational changes and problems identified during plan implementation or privacy control assessments. #privacy
  - ITEM-0576 PM-19 Privacy Program Leadership Role
    Related controls: [ITEM-0575], [ITEM-0577], [ITEM-0581], [ITEM-0582], [ITEM-0585].
    - REQ-0898 Appoint a senior agency official for privacy with the authority, mission, accountability, and resources to coordinate, develop, and implement, applicable privacy requirements and manage privacy risks through the organization-wide privacy program. #privacy
  - ITEM-0577 PM-20 Dissemination of Privacy Program Information
    Related controls: [ITEM-0016], [ITEM-0576], [ITEM-0622], [ITEM-0625], [ITEM-0628], [ITEM-0652].
    - REQ-0899 (a) Ensures that the public has access to information about organizational privacy activities and can communicate with its senior agency official for privacy; #privacy
    - REQ-0900 (b) Ensures that organizational privacy practices and reports are publicly available; and #privacy
    - REQ-0901 (c) Employs publicly facing email addresses and/or phone lines to enable the public to provide feedback and/or direct questions to privacy offices regarding privacy practices. #privacy
    - ITEM-0578 PM-20(1) Privacy Policies on Websites, Applications, and Digital Services
      - REQ-0902 (a) Are written in plain language and organized in a way that is easy to understand and navigate; #privacy
      - REQ-0903 (b) Provide information needed by the public to make an informed decision about whether and how to interact with the organization; and #privacy
      - REQ-0904 (c) Are updated whenever the organization makes a substantive change to the practices it describes and includes a time/date stamp to inform the public of the date of the most recent changes. #privacy
  - ITEM-0579 PM-21 Accounting of Disclosures
    Related controls: [ITEM-0016], [ITEM-0151], [ITEM-0612].
    - REQ-0905 (a) Develop and maintain an accurate accounting of disclosures of personally identifiable information, including: #privacy
      (a)(1) Date, nature, and purpose of each disclosure; and
      (a)(2) Name and address, or other contact information of the individual or organization to which the disclosure was made;
    - REQ-0906 (b) Retain the accounting of disclosures for the length of the time the personally identifiable information is maintained or five years after the disclosure is made, whichever is longer; and #privacy
    - REQ-0907 (c) Make the accounting of disclosures available to the individual to whom the personally identifiable information relates upon request. #privacy
  - ITEM-0580 PM-22 Personally Identifiable Information Quality Management
    Related controls: [ITEM-0581], [ITEM-0988].
    - REQ-0908 (a) Reviewing for the accuracy, relevance, timeliness, and completeness of personally identifiable information across the information life cycle; #privacy
    - REQ-0909 (b) Correcting or deleting inaccurate or outdated personally identifiable information; #privacy
    - REQ-0910 (c) Disseminating notice of corrected or deleted personally identifiable information to individuals or other appropriate entities; and #privacy
    - REQ-0911 (d) Appeals of adverse decisions on correction or deletion requests. #privacy
  - ITEM-0581 PM-23 Data Governance Body
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0576], [ITEM-0580], [ITEM-0582], [ITEM-0628], [ITEM-0918], [ITEM-0994].
    - REQ-0912 Establish a Data Governance Body consisting of [Assignment: organization-defined roles] with [Assignment: organization-defined responsibilities].
  - ITEM-0582 PM-24 Data Integrity Board
    Related controls: [ITEM-0030], [ITEM-0576], [ITEM-0581], [ITEM-0612], [ITEM-0631].
    - REQ-0913 (a) Review proposals to conduct or participate in a matching program; and #privacy
    - REQ-0914 (b) Conduct an annual review of all matching programs in which the agency has participated. #privacy
  - ITEM-0583 PM-25 Minimization of Personally Identifiable Information Used in Testing, Training, and Research
    Related controls: [ITEM-0581], [ITEM-0615], [ITEM-0658], [ITEM-0675], [ITEM-0972].
    - REQ-0915 (a) Develop, document, and implement policies and procedures that address the use of personally identifiable information for internal testing, training, and research; #privacy
    - REQ-0916 (b) Limit or minimize the amount of personally identifiable information used for internal testing, training, and research purposes; #privacy
    - REQ-0917 (c) Authorize the use of personally identifiable information when such information is required for internal testing, training, and research; and #privacy
    - REQ-0918 (d) Review and update policies and procedures [Assignment: organization-defined frequency]. #privacy
  - ITEM-0584 PM-26 Complaint Management
    Related controls: [ITEM-0431], [ITEM-0436], [ITEM-0580], [ITEM-0988].
    - REQ-0919 (a) Mechanisms that are easy to use and readily accessible by the public; #privacy
    - REQ-0920 (b) All information necessary for successfully filing complaints; #privacy
    - REQ-0921 (c) Tracking mechanisms to ensure all complaints received are reviewed and addressed within [Assignment: organization-defined time period]; #privacy
    - REQ-0922 (d) Acknowledgement of receipt of complaints, concerns, or questions from individuals within [Assignment: organization-defined time period] ; and #privacy
    - REQ-0923 (e) Response to complaints, concerns, or questions from individuals within [Assignment: organization-defined time period]. #privacy
  - ITEM-0585 PM-27 Privacy Reporting
    Related controls: [ITEM-0436], [ITEM-0576].
    - REQ-0924 (a) Develop [Assignment: organization-defined privacy reports] and disseminate to: #privacy
      (a)(1) [Assignment: organization-defined oversight bodies] to demonstrate accountability with statutory, regulatory, and policy privacy mandates; and
      (a)(2) [Assignment: organization-defined officials] and other personnel with responsibility for monitoring privacy program compliance; and
    - REQ-0925 (b) Review and update privacy reports [Assignment: organization-defined frequency]. #privacy
  - ITEM-0586 PM-28 Risk Framing
    Related controls: [ITEM-0220], [ITEM-0565], [ITEM-0636], [ITEM-0651].
    - REQ-0926 (a) Identify and document: #privacy
      (a)(1) Assumptions affecting risk assessments, risk responses, and risk monitoring;
      (a)(2) Constraints affecting risk assessments, risk responses, and risk monitoring;
      (a)(3) Priorities and trade-offs considered by the organization for managing risk; and
      (a)(4) Organizational risk tolerance;
    - REQ-0927 (b) Distribute the results of risk framing activities to [Assignment: organization-defined personnel] ; and #privacy
    - REQ-0928 (c) Review and update risk framing considerations [Assignment: organization-defined frequency]. #privacy
  - ITEM-0587 PM-29 Risk Management Program Leadership Roles
    Related controls: [ITEM-0556], [ITEM-0576].
    - REQ-0929 (a) Appoint a Senior Accountable Official for Risk Management to align organizational information security and privacy management processes with strategic, operational, and budgetary planning processes; and
    - REQ-0930 (b) Establish a Risk Executive (function) to view and analyze risk from an organization-wide perspective and ensure management of risk is consistent across the organization.
  - ITEM-0588 PM-30 Supply Chain Risk Management Strategy
    Related controls: [ITEM-0280], [ITEM-0565], [ITEM-1008], [ITEM-1009], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1023], [ITEM-1025], [ITEM-1026], [ITEM-1027], [ITEM-1030].
    - REQ-0931 (a) Develop an organization-wide strategy for managing supply chain risks associated with the development, acquisition, maintenance, and disposal of systems, system components, and system services;
    - REQ-0932 (b) Implement the supply chain risk management strategy consistently across the organization; and
    - REQ-0933 (c) Review and update the supply chain risk management strategy on [Assignment: organization-defined frequency] or as required, to address organizational changes.
    - ITEM-0589 PM-30(1) Suppliers of Critical or Mission-essential Items
      Related controls: [ITEM-0636], [ITEM-1023].
      - REQ-0934 Identify, prioritize, and assess suppliers of critical or mission-essential technologies, products, and services.
  - ITEM-0590 PM-31 Continuous Monitoring Strategy
    Related controls: [ITEM-0003], [ITEM-0062], [ITEM-0102], [ITEM-0147], [ITEM-0163], [ITEM-0195], [ITEM-0208], [ITEM-0215], [ITEM-0217], [ITEM-0220], [ITEM-0239], [ITEM-0248], [ITEM-0256], [ITEM-0282], [ITEM-0361], [ITEM-0425], [ITEM-0442], [ITEM-0444], [ITEM-0451], [ITEM-0496], [ITEM-0507], [ITEM-0528], [ITEM-0533], [ITEM-0538], [ITEM-0544], [ITEM-0558], [ITEM-0561], [ITEM-0565], [ITEM-0566], [ITEM-0568], [ITEM-0570], [ITEM-0581], [ITEM-0586], [ITEM-0607], [ITEM-0628], [ITEM-0636], [ITEM-0641], [ITEM-0651], [ITEM-0709], [ITEM-0726], [ITEM-0777], [ITEM-0782], [ITEM-0833], [ITEM-0877], [ITEM-0892], [ITEM-0913], [ITEM-0918], [ITEM-0972], [ITEM-1009], [ITEM-1015].
    - REQ-0935 (a) Establishing the following organization-wide metrics to be monitored: [Assignment: organization-defined metrics]; #privacy
    - REQ-0936 (b) Establishing [Assignment: organization-defined monitoring frequencies] and [Assignment: organization-defined assessment frequencies] for control effectiveness; #privacy
    - REQ-0937 (c) Ongoing monitoring of organizationally-defined metrics in accordance with the continuous monitoring strategy; #privacy
    - REQ-0938 (d) Correlation and analysis of information generated by control assessments and monitoring; #privacy
    - REQ-0939 (e) Response actions to address results of the analysis of control assessment and monitoring information; and #privacy
    - REQ-0940 (f) Reporting the security and privacy status of organizational systems to [Assignment: organization-defined personnel or roles] [Assignment: organization-defined frequency]. #privacy
  - ITEM-0591 PM-32 Purposing
    Related controls: [ITEM-0220], [ITEM-0544], [ITEM-0636], [ITEM-0653].
    - REQ-0941 Analyze [Assignment: organization-defined systems or system components] supporting mission essential services or functions to ensure that the information resources are being used consistent with their intended purpose.
- ITEM-0592 Personnel Security
  The personnel security family (PS) of NIST SP 800-53 Revision 5.
  - ITEM-0593 PS-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-0942 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] personnel security policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the personnel security policy and the associated personnel security controls;
    - REQ-0943 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the personnel security policy and procedures; and #low #moderate #high
    - REQ-0944 (c) Review and update the current personnel security: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0594 PS-2 Position Risk Designation
    Related controls: [ITEM-0061], [ITEM-0142], [ITEM-0492], [ITEM-0496], [ITEM-0544], [ITEM-0595], [ITEM-0604], [ITEM-0674], [ITEM-0760], [ITEM-0972].
    - REQ-0945 (a) Assign a risk designation to all organizational positions; #low #moderate #high
    - REQ-0946 (b) Establish screening criteria for individuals filling those positions; and #low #moderate #high
    - REQ-0947 (c) Review and update position risk designations [Assignment: organization-defined frequency]. #low #moderate #high
  - ITEM-0595 PS-3 Personnel Screening
    Related controls: [ITEM-0003], [ITEM-0354], [ITEM-0458], [ITEM-0492], [ITEM-0568], [ITEM-0594], [ITEM-0604], [ITEM-0607], [ITEM-0760].
    - REQ-0948 (a) Screen individuals prior to authorizing access to the system; and #low #moderate #high
    - REQ-0949 (b) Rescreen individuals in accordance with [Assignment: organization-defined conditions requiring rescreening and, where rescreening is so indicated, the frequency of rescreening]. #low #moderate #high
    - ITEM-0596 PS-3(1) Classified Information
      Related controls: [ITEM-0016], [ITEM-0030].
      - REQ-0950 Verify that individuals accessing a system processing, storing, or transmitting classified information are cleared and indoctrinated to the highest classification level of the information to which they have access on the system.
    - ITEM-0597 PS-3(2) Formal Indoctrination
      Related controls: [ITEM-0016], [ITEM-0030].
      - REQ-0951 Verify that individuals accessing a system processing, storing, or transmitting types of classified information that require formal indoctrination, are formally indoctrinated for all the relevant types of information to which they have access on the system.
    - ITEM-0598 PS-3(3) Information Requiring Special Protective Measures
      - REQ-0952 (a) Have valid access authorizations that are demonstrated by assigned official government duties; and
      - REQ-0953 (b) Satisfy [Assignment: organization-defined additional personnel screening criteria].
    - ITEM-0599 PS-3(4) Citizenship Requirements
      - REQ-0954 Verify that individuals accessing a system processing, storing, or transmitting [Assignment: organization-defined information types] meet [Assignment: organization-defined citizenship requirements].
  - ITEM-0600 PS-4 Personnel Termination
    Related controls: [ITEM-0003], [ITEM-0354], [ITEM-0492], [ITEM-0568], [ITEM-0604], [ITEM-0607].
    - REQ-0955 (a) Disable system access within [Assignment: organization-defined time period]; #low #moderate #high
    - REQ-0956 (b) Terminate or revoke any authenticators and credentials associated with the individual; #low #moderate #high
    - REQ-0957 (c) Conduct exit interviews that include a discussion of [Assignment: organization-defined information security topics]; #low #moderate #high
    - REQ-0958 (d) Retrieve all security-related organizational system-related property; and #low #moderate #high
    - REQ-0959 (e) Retain access to organizational information and systems formerly controlled by terminated individual. #low #moderate #high
    - ITEM-0601 PS-4(1) Post-employment Requirements
      - REQ-0960 (a) Notify terminated individuals of applicable, legally binding post-employment requirements for the protection of organizational information; and
      - REQ-0961 (b) Require terminated individuals to sign an acknowledgment of post-employment requirements as part of the organizational termination process.
    - ITEM-0602 PS-4(2) Automated Actions
      - REQ-0962 Use [Assignment: organization-defined automated mechanisms] to [Selection (one or more): notify [Assignment: organization-defined personnel or roles] of individual termination actions; disable access to system resources]. #high
  - ITEM-0603 PS-5 Personnel Transfer
    Related controls: [ITEM-0003], [ITEM-0354], [ITEM-0492], [ITEM-0568], [ITEM-0600], [ITEM-0607].
    - REQ-0963 (a) Review and confirm ongoing operational need for current logical and physical access authorizations to systems and facilities when individuals are reassigned or transferred to other positions within the organization; #low #moderate #high
    - REQ-0964 (b) Initiate [Assignment: organization-defined transfer or reassignment actions] within [Assignment: organization-defined time period following the formal transfer action]; #low #moderate #high
    - REQ-0965 (c) Modify access authorization as needed to correspond with any changes in operational need due to reassignment or transfer; and #low #moderate #high
    - REQ-0966 (d) Notify [Assignment: organization-defined personnel or roles] within [Assignment: organization-defined time period]. #low #moderate #high
  - ITEM-0604 PS-6 Access Agreements
    Related controls: [ITEM-0102], [ITEM-0492], [ITEM-0545], [ITEM-0594], [ITEM-0595], [ITEM-0604], [ITEM-0607], [ITEM-0608], [ITEM-0760], [ITEM-0972].
    - REQ-0967 (a) Develop and document access agreements for organizational systems; #low #moderate #high #privacy
    - REQ-0968 (b) Review and update the access agreements [Assignment: organization-defined frequency] ; and #low #moderate #high #privacy
    - REQ-0969 (c) Verify that individuals requiring access to organizational information and systems: #low #moderate #high #privacy
      (c)(1) Sign appropriate access agreements prior to being granted access; and
      (c)(2) Re-sign access agreements to maintain access to organizational systems when access agreements have been updated or [Assignment: organization-defined frequency].
    - ITEM-0605 PS-6(2) Classified Information Requiring Special Protection
      - REQ-0970 (a) Have a valid access authorization that is demonstrated by assigned official government duties;
      - REQ-0971 (b) Satisfy associated personnel security criteria; and
      - REQ-0972 (c) Have read, understood, and signed a nondisclosure agreement.
    - ITEM-0606 PS-6(3) Post-employment Requirements
      Related controls: [ITEM-0600].
      - REQ-0973 (a) Notify individuals of applicable, legally binding post-employment requirements for protection of organizational information; and
      - REQ-0974 (b) Require individuals to sign an acknowledgment of these requirements, if applicable, as part of granting initial access to covered information.
  - ITEM-0607 PS-7 External Personnel Security
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0458], [ITEM-0496], [ITEM-0594], [ITEM-0595], [ITEM-0600], [ITEM-0603], [ITEM-0604], [ITEM-0674], [ITEM-0709], [ITEM-0760].
    - REQ-0975 (a) Establish personnel security requirements, including security roles and responsibilities for external providers; #low #moderate #high
    - REQ-0976 (b) Require external providers to comply with personnel security policies and procedures established by the organization; #low #moderate #high
    - REQ-0977 (c) Document personnel security requirements; #low #moderate #high
    - REQ-0978 (d) Require external providers to notify [Assignment: organization-defined personnel or roles] of any personnel transfers or terminations of external personnel who possess organizational credentials and/or badges, or who have system privileges within [Assignment: organization-defined time period] ; and #low #moderate #high
    - REQ-0979 (e) Monitor provider compliance with personnel security requirements. #low #moderate #high
  - ITEM-0608 PS-8 Personnel Sanctions
    Related controls: [ITEM-0545], [ITEM-0568], [ITEM-0604], [ITEM-0611].
    - REQ-0980 (a) Employ a formal sanctions process for individuals failing to comply with established information security and privacy policies and procedures; and #low #moderate #high
    - REQ-0981 (b) Notify [Assignment: organization-defined personnel or roles] within [Assignment: organization-defined time period] when a formal employee sanctions process is initiated, identifying the individual sanctioned and the reason for the sanction. #low #moderate #high
  - ITEM-0609 PS-9 Position Descriptions
    - REQ-0982 Incorporate security and privacy roles and responsibilities into organizational position descriptions. #low #moderate #high
- ITEM-0610 Personally Identifiable Information Processing and Transparency
  The personally identifiable information processing and transparency family (PT) of NIST SP 800-53 Revision 5.
  - ITEM-0611 PT-1 Policy and Procedures
    - REQ-0983 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] personally identifiable information processing and transparency policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the personally identifiable information processing and transparency policy and the associated personally identifiable information processing and transparency controls;
    - REQ-0984 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the personally identifiable information processing and transparency policy and procedures; and #privacy
    - REQ-0985 (c) Review and update the current personally identifiable information processing and transparency: #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0612 PT-2 Authority to Process Personally Identifiable Information
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0287], [ITEM-0436], [ITEM-0565], [ITEM-0582], [ITEM-0611], [ITEM-0615], [ITEM-0622], [ITEM-0625], [ITEM-0636], [ITEM-0652], [ITEM-0972], [ITEM-0988].
    - REQ-0986 (a) Determine and document the [Assignment: organization-defined authority] that permits the [Assignment: organization-defined processing] of personally identifiable information; and #privacy
    - REQ-0987 (b) Restrict the [Assignment: organization-defined processing] of personally identifiable information to only that which is authorized. #privacy
    - ITEM-0613 PT-2(1) Data Tagging
      Related controls: [ITEM-0091], [ITEM-0217], [ITEM-0285], [ITEM-0559], [ITEM-0580], [ITEM-0618], [ITEM-0828], [ITEM-0892], [ITEM-0964], [ITEM-0985], [ITEM-0994].
      - REQ-0988 Attach data tags containing [Assignment: organization-defined authorized processing] to [Assignment: organization-defined elements of personally identifiable information].
    - ITEM-0614 PT-2(2) Automation
      Related controls: [ITEM-0217], [ITEM-0285], [ITEM-0559], [ITEM-0580], [ITEM-0618], [ITEM-0828], [ITEM-0892], [ITEM-0964], [ITEM-0985], [ITEM-0994].
      - REQ-0989 Manage enforcement of the authorized processing of personally identifiable information using [Assignment: organization-defined automated mechanisms].
  - ITEM-0615 PT-3 Personally Identifiable Information Processing Purposes
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0142], [ITEM-0287], [ITEM-0436], [ITEM-0565], [ITEM-0583], [ITEM-0612], [ITEM-0622], [ITEM-0625], [ITEM-0628], [ITEM-0652], [ITEM-0892], [ITEM-0972], [ITEM-0988].
    - REQ-0990 (a) Identify and document the [Assignment: organization-defined purpose(s)] for processing personally identifiable information; #privacy
    - REQ-0991 (b) Describe the purpose(s) in the public privacy notices and policies of the organization; #privacy
    - REQ-0992 (c) Restrict the [Assignment: organization-defined processing] of personally identifiable information to only that which is compatible with the identified purpose(s); and #privacy
    - REQ-0993 (d) Monitor changes in processing personally identifiable information and implement [Assignment: organization-defined mechanisms] to ensure that any changes are made in accordance with [Assignment: organization-defined requirements]. #privacy
    - ITEM-0616 PT-3(1) Data Tagging
      Related controls: [ITEM-0217], [ITEM-0285], [ITEM-0559], [ITEM-0580], [ITEM-0828], [ITEM-0892], [ITEM-0964], [ITEM-0985], [ITEM-0994].
      - REQ-0994 Attach data tags containing the following purposes to [Assignment: organization-defined elements of personally identifiable information]: [Assignment: organization-defined processing purposes].
    - ITEM-0617 PT-3(2) Automation
      Related controls: [ITEM-0217], [ITEM-0285], [ITEM-0559], [ITEM-0580], [ITEM-0828], [ITEM-0892], [ITEM-0964], [ITEM-0985], [ITEM-0994].
      - REQ-0995 Track processing purposes of personally identifiable information using [Assignment: organization-defined automated mechanisms].
  - ITEM-0618 PT-4 Consent
    Related controls: [ITEM-0091], [ITEM-0612], [ITEM-0622].
    - REQ-0996 Implement [Assignment: organization-defined tools or mechanisms] for individuals to consent to the processing of their personally identifiable information prior to its collection that facilitate individuals’ informed decision-making. #privacy
    - ITEM-0619 PT-4(1) Tailored Consent
      Related controls: [ITEM-0612].
      - REQ-0997 Provide [Assignment: organization-defined mechanisms] to allow individuals to tailor processing permissions to selected elements of personally identifiable information.
    - ITEM-0620 PT-4(2) Just-in-time Consent
      Related controls: [ITEM-0612].
      - REQ-0998 Present [Assignment: organization-defined consent mechanisms] to individuals at [Assignment: organization-defined frequency] and in conjunction with [Assignment: organization-defined personally identifiable information processing].
    - ITEM-0621 PT-4(3) Revocation
      Related controls: [ITEM-0612].
      - REQ-0999 Implement [Assignment: organization-defined tools or mechanisms] for individuals to revoke consent to the processing of their personally identifiable information.
  - ITEM-0622 PT-5 Privacy Notice
    Related controls: [ITEM-0577], [ITEM-0580], [ITEM-0612], [ITEM-0615], [ITEM-0618], [ITEM-0628], [ITEM-0636], [ITEM-0887], [ITEM-0988].
    - REQ-1000 (a) Is available to individuals upon first interacting with an organization, and subsequently at [Assignment: organization-defined frequency]; #privacy
    - REQ-1001 (b) Is clear and easy-to-understand, expressing information about personally identifiable information processing in plain language; #privacy
    - REQ-1002 (c) Identifies the authority that authorizes the processing of personally identifiable information; #privacy
    - REQ-1003 (d) Identifies the purposes for which personally identifiable information is to be processed; and #privacy
    - REQ-1004 (e) Includes [Assignment: organization-defined information]. #privacy
    - ITEM-0623 PT-5(1) Just-in-time Notice
      Related controls: [ITEM-0579].
      - REQ-1005 Present notice of personally identifiable information processing to individuals at a time and location where the individual provides personally identifiable information or in conjunction with a data action, or [Assignment: organization-defined frequency].
    - ITEM-0624 PT-5(2) Privacy Act Statements
      Related controls: [ITEM-0625].
      - REQ-1006 Include Privacy Act statements on forms that collect information that will be maintained in a Privacy Act system of records, or provide Privacy Act statements on separate forms that can be retained by individuals. #privacy
  - ITEM-0625 PT-6 System of Records Notice
    Related controls: [ITEM-0016], [ITEM-0577], [ITEM-0612], [ITEM-0615], [ITEM-0622].
    - REQ-1007 (a) Draft system of records notices in accordance with OMB guidance and submit new and significantly modified system of records notices to the OMB and appropriate congressional committees for advance review; #privacy
    - REQ-1008 (b) Publish system of records notices in the Federal Register; and #privacy
    - REQ-1009 (c) Keep system of records notices accurate, up-to-date, and scoped in accordance with policy. #privacy
    - ITEM-0626 PT-6(1) Routine Uses
      - REQ-1010 Review all routine uses published in the system of records notice at [Assignment: organization-defined frequency] to ensure continued accuracy, and to ensure that routine uses continue to be compatible with the purpose for which the information was collected. #privacy
    - ITEM-0627 PT-6(2) Exemption Rules
      - REQ-1011 Review all Privacy Act exemptions claimed for the system of records at [Assignment: organization-defined frequency] to ensure they remain appropriate and necessary in accordance with law, that they have been promulgated as regulations, and that they are accurately described in the system of records notice. #privacy
  - ITEM-0628 PT-7 Specific Categories of Personally Identifiable Information
    Related controls: [ITEM-0436], [ITEM-0612], [ITEM-0615], [ITEM-0636].
    - REQ-1012 Apply [Assignment: organization-defined processing conditions] for specific categories of personally identifiable information. #privacy
    - ITEM-0629 PT-7(1) Social Security Numbers
      Related controls: [ITEM-0354].
      - REQ-1013 (a) Eliminate unnecessary collection, maintenance, and use of Social Security numbers, and explore alternatives to their use as a personal identifier; #privacy
      - REQ-1014 (b) Do not deny any individual any right, benefit, or privilege provided by law because of such individual’s refusal to disclose his or her Social Security number; and #privacy
      - REQ-1015 (c) Inform any individual who is asked to disclose his or her Social Security number whether that disclosure is mandatory or voluntary, by what statutory or other authority such number is solicited, and what uses will be made of it. #privacy
    - ITEM-0630 PT-7(2) First Amendment Information
      - REQ-1016 Prohibit the processing of information describing how any individual exercises rights guaranteed by the First Amendment unless expressly authorized by statute or by the individual or unless pertinent to and within the scope of an authorized law enforcement activity. #privacy
  - ITEM-0631 PT-8 Computer Matching Requirements
    Related controls: [ITEM-0582].
    - REQ-1017 (a) Obtain approval from the Data Integrity Board to conduct the matching program; #privacy
    - REQ-1018 (b) Develop and enter into a computer matching agreement; #privacy
    - REQ-1019 (c) Publish a matching notice in the Federal Register; #privacy
    - REQ-1020 (d) Independently verify the information produced by the matching program before taking adverse action against an individual, if required; and #privacy
    - REQ-1021 (e) Provide individuals with notice and an opportunity to contest the findings before taking adverse action against an individual. #privacy
- ITEM-0632 Risk Assessment
  The risk assessment family (RA) of NIST SP 800-53 Revision 5.
  - ITEM-0633 RA-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0972].
    - REQ-1022 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] risk assessment policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the risk assessment policy and the associated risk assessment controls;
    - REQ-1023 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the risk assessment policy and procedures; and #low #moderate #high #privacy
    - REQ-1024 (c) Review and update the current risk assessment: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0634 RA-2 Security Categorization
    Related controls: [ITEM-0269], [ITEM-0473], [ITEM-0544], [ITEM-0552], [ITEM-0553], [ITEM-0562], [ITEM-0636], [ITEM-0641], [ITEM-0651], [ITEM-0652], [ITEM-0675], [ITEM-0782], [ITEM-0877], [ITEM-0972].
    - REQ-1025 (a) Categorize the system and information it processes, stores, and transmits; #low #moderate #high
    - REQ-1026 (b) Document the security categorization results, including supporting rationale, in the security plan for the system; and #low #moderate #high
    - REQ-1027 (c) Verify that the authorizing official or authorizing official designated representative reviews and approves the security categorization decision. #low #moderate #high
    - ITEM-0635 RA-2(1) Impact-level Prioritization
      - REQ-1028 Conduct an impact-level prioritization of organizational systems to obtain additional granularity on system impact levels.
  - ITEM-0636 RA-3 Risk Assessment
    Related controls: [ITEM-0212], [ITEM-0217], [ITEM-0248], [ITEM-0287], [ITEM-0308], [ITEM-0312], [ITEM-0379], [ITEM-0458], [ITEM-0496], [ITEM-0512], [ITEM-0535], [ITEM-0544], [ITEM-0552], [ITEM-0553], [ITEM-0564], [ITEM-0565], [ITEM-0586], [ITEM-0612], [ITEM-0628], [ITEM-0634], [ITEM-0641], [ITEM-0651], [ITEM-0675], [ITEM-0709], [ITEM-0877], [ITEM-0972].
    - REQ-1029 (a) Conduct a risk assessment, including: #low #moderate #high #privacy
      (a)(1) Identifying threats to and vulnerabilities in the system;
      (a)(2) Determining the likelihood and magnitude of harm from unauthorized access, use, disclosure, disruption, modification, or destruction of the system, the information it processes, stores, or transmits, and any related information; and
      (a)(3) Determining the likelihood and impact of adverse effects on individuals arising from the processing of personally identifiable information;
    - REQ-1030 (b) Integrate risk assessment results and risk management decisions from the organization and mission or business process perspectives with system-level risk assessments; #low #moderate #high #privacy
    - REQ-1031 (c) Document risk assessment results in [Selection: security and privacy plans; risk assessment report; [Assignment: organization-defined document] ]; #low #moderate #high #privacy
    - REQ-1032 (d) Review risk assessment results [Assignment: organization-defined frequency]; #low #moderate #high #privacy
    - REQ-1033 (e) Disseminate risk assessment results to [Assignment: organization-defined personnel or roles] ; and #low #moderate #high #privacy
    - REQ-1034 (f) Update the risk assessment [Assignment: organization-defined frequency] or when there are significant changes to the system, its environment of operation, or other conditions that may impact the security or privacy state of the system. #low #moderate #high #privacy
    - ITEM-0637 RA-3(1) Supply Chain Risk Assessment
      Related controls: [ITEM-0634], [ITEM-0653], [ITEM-0574], [ITEM-0588], [ITEM-1009].
      - REQ-1035 (a) Assess supply chain risks associated with [Assignment: organization-defined systems, system components, and system services] ; and #low #moderate #high
      - REQ-1036 (b) Update the supply chain risk assessment [Assignment: organization-defined frequency] , when there are significant changes to the relevant supply chain, or when changes to the system, environments of operation, or other conditions may necessitate a change in the supply chain. #low #moderate #high
    - ITEM-0638 RA-3(2) Use of All-source Intelligence
      - REQ-1037 Use all-source intelligence to assist in the analysis of risk.
    - ITEM-0639 RA-3(3) Dynamic Threat Awareness
      Related controls: [ITEM-0135].
      - REQ-1038 Determine the current cyber threat environment on an ongoing basis using [Assignment: organization-defined means].
    - ITEM-0640 RA-3(4) Predictive Cyber Analytics
      - REQ-1039 Employ the following advanced automation and analytics capabilities to predict and identify risks to [Assignment: organization-defined systems or system components]: [Assignment: organization-defined advanced automation and analytics capabilities].
  - ITEM-0641 RA-5 Vulnerability Monitoring and Scanning
    Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0226], [ITEM-0234], [ITEM-0248], [ITEM-0256], [ITEM-0269], [ITEM-0634], [ITEM-0636], [ITEM-0726], [ITEM-0736], [ITEM-0877], [ITEM-0906], [ITEM-0913], [ITEM-0918], [ITEM-0947], [ITEM-1030].
    - REQ-1040 (a) Monitor and scan for vulnerabilities in the system and hosted applications [Assignment: organization-defined frequency and/or randomly in accordance with organization-defined process] and when new vulnerabilities potentially affecting the system are identified and reported; #low #moderate #high
    - REQ-1041 (b) Employ vulnerability monitoring tools and techniques that facilitate interoperability among tools and automate parts of the vulnerability management process by using standards for: #low #moderate #high
      (b)(1) Enumerating platforms, software flaws, and improper configurations;
      (b)(2) Formatting checklists and test procedures; and
      (b)(3) Measuring vulnerability impact;
    - REQ-1042 (c) Analyze vulnerability scan reports and results from vulnerability monitoring; #low #moderate #high
    - REQ-1043 (d) Remediate legitimate vulnerabilities [Assignment: organization-defined response times] in accordance with an organizational assessment of risk; #low #moderate #high
    - REQ-1044 (e) Share information obtained from the vulnerability monitoring process and control assessments with [Assignment: organization-defined personnel or roles] to help eliminate similar vulnerabilities in other systems; and #low #moderate #high
    - REQ-1045 (f) Employ vulnerability monitoring tools that include the capability to readily update the vulnerabilities to be scanned. #low #moderate #high
    - ITEM-0642 RA-5(2) Update Vulnerabilities to Be Scanned
      Related controls: [ITEM-0942].
      - REQ-1046 Update the system vulnerabilities to be scanned [Selection (one or more): [Assignment: organization-defined frequency] ; prior to a new scan; when new vulnerabilities are identified and reported]. #low #moderate #high
    - ITEM-0643 RA-5(3) Breadth and Depth of Coverage
      - REQ-1047 Define the breadth and depth of vulnerability scanning coverage.
    - ITEM-0644 RA-5(4) Discoverable Information
      Related controls: [ITEM-0195], [ITEM-0849].
      - REQ-1048 Determine information about the system that is discoverable and take [Assignment: organization-defined corrective actions]. #high
    - ITEM-0645 RA-5(5) Privileged Access
      - REQ-1049 Implement privileged access authorization to [Assignment: organization-defined system components] for [Assignment: organization-defined vulnerability scanning activities]. #moderate #high
    - ITEM-0646 RA-5(6) Automated Trend Analyses
      - REQ-1050 Compare the results of multiple vulnerability scans using [Assignment: organization-defined automated mechanisms].
    - ITEM-0647 RA-5(8) Review Historic Audit Logs
      Related controls: [ITEM-0163], [ITEM-0188].
      - REQ-1051 Review historic audit logs to determine if a vulnerability identified in a [Assignment: organization-defined system] has been previously exploited within an [Assignment: organization-defined time period].
    - ITEM-0648 RA-5(10) Correlate Scanning Information
      - REQ-1052 Correlate the output from vulnerability scanning tools to determine the presence of multi-vulnerability and multi-hop attack vectors.
    - ITEM-0649 RA-5(11) Public Disclosure Program
      - REQ-1053 Establish a public reporting channel for receiving reports of vulnerabilities in organizational systems and system components. #low #moderate #high
  - ITEM-0650 RA-6 Technical Surveillance Countermeasures Survey
    - REQ-1054 Employ a technical surveillance countermeasures survey at [Assignment: organization-defined locations] [Selection (one or more): [Assignment: organization-defined frequency] ; when [Assignment: organization-defined events or indicators] ].
  - ITEM-0651 RA-7 Risk Response
    Related controls: [ITEM-0215], [ITEM-0436], [ITEM-0558], [ITEM-0586], [ITEM-0634], [ITEM-0636], [ITEM-1009].
    - REQ-1055 Respond to findings from security and privacy assessments, monitoring, and audits in accordance with organizational risk tolerance. #low #moderate #high #privacy
  - ITEM-0652 RA-8 Privacy Impact Assessments
    Related controls: [ITEM-0248], [ITEM-0278], [ITEM-0287], [ITEM-0612], [ITEM-0615], [ITEM-0622], [ITEM-0633], [ITEM-0634], [ITEM-0636], [ITEM-0651].
    - REQ-1056 (a) Developing or procuring information technology that processes personally identifiable information; and #privacy
    - REQ-1057 (b) Initiating a new collection of personally identifiable information that: #privacy
      (b)(1) Will be processed using information technology; and
      (b)(2) Includes personally identifiable information permitting the physical or virtual (online) contacting of a specific individual, if identical questions have been posed to, or identical reporting requirements imposed on, ten or more individuals, other than agencies, instrumentalities, or employees of the federal government.
  - ITEM-0653 RA-9 Criticality Analysis
    Related controls: [ITEM-0291], [ITEM-0544], [ITEM-0548], [ITEM-0553], [ITEM-0555], [ITEM-0567], [ITEM-0634], [ITEM-0675], [ITEM-0736], [ITEM-0759], [ITEM-1020].
    - REQ-1058 Identify critical system components and functions by performing a criticality analysis for [Assignment: organization-defined systems, system components, or system services] at [Assignment: organization-defined decision points in the system development life cycle]. #moderate #high
  - ITEM-0654 RA-10 Threat Hunting
    Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0226], [ITEM-0636], [ITEM-0641], [ITEM-0650], [ITEM-0918].
    - REQ-1059 (a) Establish and maintain a cyber threat hunting capability to:
      (a)(1) Search for indicators of compromise in organizational systems; and
      (a)(2) Detect, track, and disrupt threats that evade existing controls; and
    - REQ-1060 (b) Employ the threat hunting capability [Assignment: organization-defined frequency].
- ITEM-0655 System and Services Acquisition
  The system and services acquisition family (SA) of NIST SP 800-53 Revision 5.
  - ITEM-0656 SA-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0675], [ITEM-0972].
    - REQ-1061 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] system and services acquisition policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the system and services acquisition policy and the associated system and services acquisition controls;
    - REQ-1062 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the system and services acquisition policy and procedures; and #low #moderate #high #privacy
    - REQ-1063 (c) Review and update the current system and services acquisition: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0657 SA-2 Allocation of Resources
    Related controls: [ITEM-0547], [ITEM-0557], [ITEM-0567], [ITEM-0709], [ITEM-1011], [ITEM-1020].
    - REQ-1064 (a) Determine the high-level information security and privacy requirements for the system or system service in mission and business process planning; #low #moderate #high #privacy
    - REQ-1065 (b) Determine, document, and allocate the resources required to protect the system or system service as part of the organizational capital planning and investment control process; and #low #moderate #high #privacy
    - REQ-1066 (c) Establish a discrete line item for information security and privacy in organizational programming and budgeting documentation. #low #moderate #high #privacy
  - ITEM-0658 SA-3 System Development Life Cycle
    Related controls: [ITEM-0142], [ITEM-0548], [ITEM-0562], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0726], [ITEM-0736], [ITEM-0749], [ITEM-0761], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1027].
    - REQ-1067 (a) Acquire, develop, and manage the system using [Assignment: organization-defined system-development life cycle] that incorporates information security and privacy considerations; #low #moderate #high #privacy
    - REQ-1068 (b) Define and document information security and privacy roles and responsibilities throughout the system development life cycle; #low #moderate #high #privacy
    - REQ-1069 (c) Identify individuals having information security and privacy roles and responsibilities; and #low #moderate #high #privacy
    - REQ-1070 (d) Integrate the organizational information security and privacy risk management process into system development life cycle activities. #low #moderate #high #privacy
    - ITEM-0659 SA-3(1) Manage Preproduction Environment
      Related controls: [ITEM-0234], [ITEM-0248], [ITEM-0636], [ITEM-0653], [ITEM-0662].
      - REQ-1071 Protect system preproduction environments commensurate with risk throughout the system development life cycle for the system, system component, or system service.
    - ITEM-0660 SA-3(2) Use of Live or Operational Data
      Related controls: [ITEM-0583], [ITEM-0636].
      - REQ-1072 (a) Approve, document, and control the use of live data in preproduction environments for the system, system component, or system service; and
      - REQ-1073 (b) Protect preproduction environments for the system, system component, or system service at the same impact or classification level as any live data in use within the preproduction environments.
    - ITEM-0661 SA-3(3) Technology Refresh
      Related controls: [ITEM-0464].
      - REQ-1074 Plan for and implement a technology refresh schedule for the system throughout the system development life cycle.
  - ITEM-0662 SA-4 Acquisition Process
    Related controls: [ITEM-0256], [ITEM-0269], [ITEM-0607], [ITEM-0658], [ITEM-0674], [ITEM-0675], [ITEM-0726], [ITEM-0736], [ITEM-0748], [ITEM-0749], [ITEM-0760], [ITEM-1011], [ITEM-1020].
    - REQ-1075 (a) Security and privacy functional requirements; #low #moderate #high #privacy
    - REQ-1076 (b) Strength of mechanism requirements; #low #moderate #high #privacy
    - REQ-1077 (c) Security and privacy assurance requirements; #low #moderate #high #privacy
    - REQ-1078 (d) Controls needed to satisfy the security and privacy requirements. #low #moderate #high #privacy
    - REQ-1079 (e) Security and privacy documentation requirements; #low #moderate #high #privacy
    - REQ-1080 (f) Requirements for protecting security and privacy documentation; #low #moderate #high #privacy
    - REQ-1081 (g) Description of the system development environment and environment in which the system is intended to operate; #low #moderate #high #privacy
    - REQ-1082 (h) Allocation of responsibility or identification of parties responsible for information security, privacy, and supply chain risk management; and #low #moderate #high #privacy
    - REQ-1083 (i) Acceptance criteria. #low #moderate #high #privacy
    - ITEM-0663 SA-4(1) Functional Properties of Controls
      - REQ-1084 Require the developer of the system, system component, or system service to provide a description of the functional properties of the controls to be implemented. #moderate #high
    - ITEM-0664 SA-4(2) Design and Implementation Information for Controls
      - REQ-1085 Require the developer of the system, system component, or system service to provide design and implementation information for the controls that includes: [Selection (one or more): security-relevant external system interfaces; high-level design; low-level design; source code or hardware schematics; [Assignment: organization-defined design and implementation information] ] at [Assignment: organization-defined level of detail]. #moderate #high
    - ITEM-0665 SA-4(3) Development Methods, Techniques, and Practices
      - REQ-1086 (a) [Assignment: organization-defined systems engineering methods];
      - REQ-1087 (b) [Selection (one or more): [Assignment: organization-defined system security engineering methods] ; [Assignment: organization-defined privacy engineering methods] ] ; and
      - REQ-1088 (c) [Selection (one or more): [Assignment: organization-defined software development methods] ; [Assignment: organization-defined testing, evaluation, assessment, verification, and validation methods] ; [Assignment: organization-defined quality control processes] ].
    - ITEM-0666 SA-4(5) System, Component, and Service Configurations
      - REQ-1089 (a) Deliver the system, component, or service with [Assignment: organization-defined security configurations] implemented; and #high
      - REQ-1090 (b) Use the configurations as the default for any subsequent system, component, or service reinstallation or upgrade. #high
    - ITEM-0667 SA-4(6) Use of Information Assurance Products
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-1091 (a) Employ only government off-the-shelf or commercial off-the-shelf information assurance and information assurance-enabled information technology products that compose an NSA-approved solution to protect classified information when the networks used to transmit the information are at a lower classification level than the information being transmitted; and
      - REQ-1092 (b) Ensure that these products have been evaluated and/or validated by NSA or in accordance with NSA-approved procedures.
    - ITEM-0668 SA-4(7) NIAP-approved Protection Profiles 
      Related controls: [ITEM-0378], [ITEM-0818], [ITEM-0823].
      - REQ-1093 (a) Limit the use of commercially provided information assurance and information assurance-enabled information technology products to those products that have been successfully evaluated against a National Information Assurance partnership (NIAP)-approved Protection Profile for a specific technology type, if such a profile exists; and
      - REQ-1094 (b) Require, if no NIAP-approved Protection Profile exists for a specific technology type but a commercially provided information technology product relies on cryptographic functionality to enforce its security policy, that the cryptographic module is FIPS-validated or NSA-approved.
    - ITEM-0669 SA-4(8) Continuous Monitoring Plan for Controls
      Related controls: [ITEM-0220].
      - REQ-1095 Require the developer of the system, system component, or system service to produce a plan for continuous monitoring of control effectiveness that is consistent with the continuous monitoring program of the organization.
    - ITEM-0670 SA-4(9) Functions, Ports, Protocols, and Services in Use
      Related controls: [ITEM-0259], [ITEM-0709].
      - REQ-1096 Require the developer of the system, system component, or system service to identify the functions, ports, protocols, and services intended for organizational use. #moderate #high
    - ITEM-0671 SA-4(10) Use of Approved PIV Products
      Related controls: [ITEM-0341], [ITEM-0379], [ITEM-0565].
      - REQ-1097 Employ only information technology products on the FIPS 201-approved products list for Personal Identity Verification (PIV) capability implemented within organizational systems. #low #moderate #high
    - ITEM-0672 SA-4(11) System of Records
      Related controls: [ITEM-0625].
      - REQ-1098 Include [Assignment: organization-defined Privacy Act requirements] in the acquisition contract for the operation of a system of records on behalf of an organization to accomplish an organizational mission or function.
    - ITEM-0673 SA-4(12) Data Ownership
      - REQ-1099 (a) Include organizational data ownership requirements in the acquisition contract; and
      - REQ-1100 (b) Require all data to be removed from the contractor’s system and returned to the organization within [Assignment: organization-defined time frame].
  - ITEM-0674 SA-5 System Documentation
    Related controls: [ITEM-0248], [ITEM-0256], [ITEM-0259], [ITEM-0269], [ITEM-0544], [ITEM-0545], [ITEM-0548], [ITEM-0594], [ITEM-0658], [ITEM-0662], [ITEM-0675], [ITEM-0709], [ITEM-0718], [ITEM-0726], [ITEM-0736], [ITEM-0748], [ITEM-0749], [ITEM-0972], [ITEM-1011].
    - REQ-1101 (a) Obtain or develop administrator documentation for the system, system component, or system service that describes: #low #moderate #high
      (a)(1) Secure configuration, installation, and operation of the system, component, or service;
      (a)(2) Effective use and maintenance of security and privacy functions and mechanisms; and
      (a)(3) Known vulnerabilities regarding configuration and use of administrative or privileged functions;
    - REQ-1102 (b) Obtain or develop user documentation for the system, system component, or system service that describes: #low #moderate #high
      (b)(1) User-accessible security and privacy functions and mechanisms and how to effectively use those functions and mechanisms;
      (b)(2) Methods for user interaction, which enables individuals to use the system, component, or service in a more secure manner and protect individual privacy; and
      (b)(3) User responsibilities in maintaining the security of the system, component, or service and privacy of individuals;
    - REQ-1103 (c) Document attempts to obtain system, system component, or system service documentation when such documentation is either unavailable or nonexistent and take [Assignment: organization-defined actions] in response; and #low #moderate #high
    - REQ-1104 (d) Distribute documentation to [Assignment: organization-defined personnel or roles]. #low #moderate #high
  - ITEM-0675 SA-8 Security and Privacy Engineering Principles
    Related controls: [ITEM-0548], [ITEM-0562], [ITEM-0634], [ITEM-0636], [ITEM-0653], [ITEM-0658], [ITEM-0662], [ITEM-0736], [ITEM-0749], [ITEM-0759], [ITEM-0766], [ITEM-0769], [ITEM-0866], [ITEM-0878], [ITEM-1009], [ITEM-1011], [ITEM-1015], [ITEM-1020].
    - REQ-1105 Apply the following systems security and privacy engineering principles in the specification, design, development, implementation, and modification of the system and system components: [Assignment: organization-defined systems security and privacy engineering principles]. #low #moderate #high
    - ITEM-0676 SA-8(1) Clear Abstractions
      - REQ-1106 Implement the security design principle of clear abstractions.
    - ITEM-0677 SA-8(2) Least Common Mechanism
      - REQ-1107 Implement the security design principle of least common mechanism in [Assignment: organization-defined systems or system components].
    - ITEM-0678 SA-8(3) Modularity and Layering
      Related controls: [ITEM-0766], [ITEM-0769].
      - REQ-1108 Implement the security design principles of modularity and layering in [Assignment: organization-defined systems or system components].
    - ITEM-0679 SA-8(4) Partially Ordered Dependencies
      - REQ-1109 Implement the security design principle of partially ordered dependencies in [Assignment: organization-defined systems or system components].
    - ITEM-0680 SA-8(5) Efficiently Mediated Access
      Related controls: [ITEM-0132].
      - REQ-1110 Implement the security design principle of efficiently mediated access in [Assignment: organization-defined systems or system components].
    - ITEM-0681 SA-8(6) Minimized Sharing
      Related controls: [ITEM-0862].
      - REQ-1111 Implement the security design principle of minimized sharing in [Assignment: organization-defined systems or system components].
    - ITEM-0682 SA-8(7) Reduced Complexity
      - REQ-1112 Implement the security design principle of reduced complexity in [Assignment: organization-defined systems or system components].
    - ITEM-0683 SA-8(8) Secure Evolvability
      Related controls: [ITEM-0239].
      - REQ-1113 Implement the security design principle of secure evolvability in [Assignment: organization-defined systems or system components].
    - ITEM-0684 SA-8(9) Trusted Components
      - REQ-1114 Implement the security design principle of trusted components in [Assignment: organization-defined systems or system components].
    - ITEM-0685 SA-8(10) Hierarchical Trust
      - REQ-1115 Implement the security design principle of hierarchical trust in [Assignment: organization-defined systems or system components].
    - ITEM-0686 SA-8(11) Inverse Modification Threshold
      - REQ-1116 Implement the security design principle of inverse modification threshold in [Assignment: organization-defined systems or system components].
    - ITEM-0687 SA-8(12) Hierarchical Protection
      - REQ-1117 Implement the security design principle of hierarchical protection in [Assignment: organization-defined systems or system components].
    - ITEM-0688 SA-8(13) Minimized Security Elements
      - REQ-1118 Implement the security design principle of minimized security elements in [Assignment: organization-defined systems or system components].
    - ITEM-0689 SA-8(14) Least Privilege
      Related controls: [ITEM-0062], [ITEM-0259].
      - REQ-1119 Implement the security design principle of least privilege in [Assignment: organization-defined systems or system components].
    - ITEM-0690 SA-8(15) Predicate Permission
      Related controls: [ITEM-0061].
      - REQ-1120 Implement the security design principle of predicate permission in [Assignment: organization-defined systems or system components].
    - ITEM-0691 SA-8(16) Self-reliant Trustworthiness
      - REQ-1121 Implement the security design principle of self-reliant trustworthiness in [Assignment: organization-defined systems or system components].
    - ITEM-0692 SA-8(17) Secure Distributed Composition
      - REQ-1122 Implement the security design principle of secure distributed composition in [Assignment: organization-defined systems or system components].
    - ITEM-0693 SA-8(18) Trusted Communications Channels
      Related controls: [ITEM-0809], [ITEM-0818], [ITEM-0823].
      - REQ-1123 Implement the security design principle of trusted communications channels in [Assignment: organization-defined systems or system components].
    - ITEM-0694 SA-8(19) Continuous Protection
      Related controls: [ITEM-0132].
      - REQ-1124 Implement the security design principle of continuous protection in [Assignment: organization-defined systems or system components].
    - ITEM-0695 SA-8(20) Secure Metadata Management
      - REQ-1125 Implement the security design principle of secure metadata management in [Assignment: organization-defined systems or system components].
    - ITEM-0696 SA-8(21) Self-analysis
      Related controls: [ITEM-0220].
      - REQ-1126 Implement the security design principle of self-analysis in [Assignment: organization-defined systems or system components].
    - ITEM-0697 SA-8(22) Accountability and Traceability
      Related controls: [ITEM-0062], [ITEM-0151], [ITEM-0152], [ITEM-0163], [ITEM-0175], [ITEM-0183], [ITEM-0190], [ITEM-0341], [ITEM-0409].
      - REQ-1127 Implement the security design principle of accountability and traceability in [Assignment: organization-defined systems or system components].
    - ITEM-0698 SA-8(23) Secure Defaults
      Related controls: [ITEM-0234], [ITEM-0256], [ITEM-0662].
      - REQ-1128 Implement the security design principle of secure defaults in [Assignment: organization-defined systems or system components].
    - ITEM-0699 SA-8(24) Secure Failure and Recovery
      Related controls: [ITEM-0332], [ITEM-0337], [ITEM-0782], [ITEM-0809], [ITEM-0847], [ITEM-0976].
      - REQ-1129 Implement the security design principle of secure failure and recovery in [Assignment: organization-defined systems or system components].
    - ITEM-0700 SA-8(25) Economic Security
      Related controls: [ITEM-0636].
      - REQ-1130 Implement the security design principle of economic security in [Assignment: organization-defined systems or system components].
    - ITEM-0701 SA-8(26) Performance Security
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0906], [ITEM-0947].
      - REQ-1131 Implement the security design principle of performance security in [Assignment: organization-defined systems or system components].
    - ITEM-0702 SA-8(27) Human Factored Security
      - REQ-1132 Implement the security design principle of human factored security in [Assignment: organization-defined systems or system components].
    - ITEM-0703 SA-8(28) Acceptable Security
      - REQ-1133 Implement the security design principle of acceptable security in [Assignment: organization-defined systems or system components].
    - ITEM-0704 SA-8(29) Repeatable and Documented Procedures
      Related controls: [ITEM-0233], [ITEM-0656], [ITEM-0718], [ITEM-0726], [ITEM-0736], [ITEM-0749], [ITEM-0765], [ITEM-0905].
      - REQ-1134 Implement the security design principle of repeatable and documented procedures in [Assignment: organization-defined systems or system components].
    - ITEM-0705 SA-8(30) Procedural Rigor
      - REQ-1135 Implement the security design principle of procedural rigor in [Assignment: organization-defined systems or system components].
    - ITEM-0706 SA-8(31) Secure System Modification
      Related controls: [ITEM-0239], [ITEM-0248].
      - REQ-1136 Implement the security design principle of secure system modification in [Assignment: organization-defined systems or system components].
    - ITEM-0707 SA-8(32) Sufficient Documentation
      Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0674].
      - REQ-1137 Implement the security design principle of sufficient documentation in [Assignment: organization-defined systems or system components].
    - ITEM-0708 SA-8(33) Minimization
      Related controls: [ITEM-0512], [ITEM-0583], [ITEM-0887], [ITEM-0972].
      - REQ-1138 Implement the privacy principle of minimization using [Assignment: organization-defined processes]. #privacy
  - ITEM-0709 SA-9 External System Services
    Related controls: [ITEM-0118], [ITEM-0212], [ITEM-0291], [ITEM-0409], [ITEM-0431], [ITEM-0552], [ITEM-0553], [ITEM-0607], [ITEM-0657], [ITEM-0662], [ITEM-1011], [ITEM-1020].
    - REQ-1139 (a) Require that providers of external system services comply with organizational security and privacy requirements and employ the following controls: [Assignment: organization-defined controls]; #low #moderate #high #privacy
    - REQ-1140 (b) Define and document organizational oversight and user roles and responsibilities with regard to external system services; and #low #moderate #high #privacy
    - REQ-1141 (c) Employ the following processes, methods, and techniques to monitor control compliance by external service providers on an ongoing basis: [Assignment: organization-defined processes, methods, and techniques]. #low #moderate #high #privacy
    - ITEM-0710 SA-9(1) Risk Assessments and Organizational Approvals
      Related controls: [ITEM-0217], [ITEM-0636], [ITEM-0652].
      - REQ-1142 (a) Conduct an organizational assessment of risk prior to the acquisition or outsourcing of information security services; and
      - REQ-1143 (b) Verify that the acquisition or outsourcing of dedicated information security services is approved by [Assignment: organization-defined personnel or roles].
    - ITEM-0711 SA-9(2) Identification of Functions, Ports, Protocols, and Services
      Related controls: [ITEM-0256], [ITEM-0259].
      - REQ-1144 Require providers of the following external system services to identify the functions, ports, protocols, and other services required for the use of such services: [Assignment: organization-defined external system services]. #moderate #high
    - ITEM-0712 SA-9(3) Establish and Maintain Trust Relationship with Providers
      Related controls: [ITEM-1009].
      - REQ-1145 Establish, document, and maintain trust relationships with external service providers based on the following requirements, properties, factors, or conditions: [Assignment: organization-defined security and privacy requirements, properties, factors, or conditions defining acceptable trust relationships].
    - ITEM-0713 SA-9(4) Consistent Interests of Consumers and Providers
      - REQ-1146 Take the following actions to verify that the interests of [Assignment: organization-defined external service providers] are consistent with and reflect organizational interests: [Assignment: organization-defined actions].
    - ITEM-0714 SA-9(5) Processing, Storage, and Service Location
      Related controls: [ITEM-0674], [ITEM-1015].
      - REQ-1147 Restrict the location of [Selection (one or more): information processing; information or data; system services] to [Assignment: organization-defined locations] based on [Assignment: organization-defined requirements].
    - ITEM-0715 SA-9(6) Organization-controlled Cryptographic Keys
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0918].
      - REQ-1148 Maintain exclusive control of cryptographic keys for encrypted material stored or transmitted through an external system.
    - ITEM-0716 SA-9(7) Organization-controlled Integrity Checking
      Related controls: [ITEM-0947].
      - REQ-1149 Provide the capability to check the integrity of information while it resides in the external system.
    - ITEM-0717 SA-9(8) Processing and Storage Location — U.S. Jurisdiction
      Related controls: [ITEM-0674], [ITEM-1015].
      - REQ-1150 Restrict the geographic location of information processing and data storage to facilities located within in the legal jurisdictional boundary of the United States.
  - ITEM-0718 SA-10 Developer Configuration Management
    Related controls: [ITEM-0234], [ITEM-0239], [ITEM-0248], [ITEM-0259], [ITEM-0278], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0736], [ITEM-0906], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1023].
    - REQ-1151 (a) Perform configuration management during system, component, or service [Selection (one or more): design; development; implementation; operation; disposal]; #moderate #high
    - REQ-1152 (b) Document, manage, and control the integrity of changes to [Assignment: organization-defined configuration items]; #moderate #high
    - REQ-1153 (c) Implement only organization-approved changes to the system, component, or service; #moderate #high
    - REQ-1154 (d) Document approved changes to the system, component, or service and the potential security and privacy impacts of such changes; and #moderate #high
    - REQ-1155 (e) Track security flaws and flaw resolution within the system, component, or service and report findings to [Assignment: organization-defined personnel]. #moderate #high
    - ITEM-0719 SA-10(1) Software and Firmware Integrity Verification
      Related controls: [ITEM-0947], [ITEM-1030].
      - REQ-1156 Require the developer of the system, system component, or system service to enable integrity verification of software and firmware components.
    - ITEM-0720 SA-10(2) Alternative Configuration Management Processes
      - REQ-1157 Provide an alternate configuration management process using organizational personnel in the absence of a dedicated developer configuration management team.
    - ITEM-0721 SA-10(3) Hardware Integrity Verification
      Related controls: [ITEM-0947].
      - REQ-1158 Require the developer of the system, system component, or system service to enable integrity verification of hardware components.
    - ITEM-0722 SA-10(4) Trusted Generation
      - REQ-1159 Require the developer of the system, system component, or system service to employ tools for comparing newly generated versions of security-relevant hardware descriptions, source code, and object code with previous versions.
    - ITEM-0723 SA-10(5) Mapping Integrity for Version Control
      - REQ-1160 Require the developer of the system, system component, or system service to maintain the integrity of the mapping between the master build data describing the current version of security-relevant hardware, software, and firmware and the on-site master copy of the data for the current version.
    - ITEM-0724 SA-10(6) Trusted Distribution
      - REQ-1161 Require the developer of the system, system component, or system service to execute procedures for ensuring that security-relevant hardware, software, and firmware updates distributed to the organization are exactly as specified by the master copies.
    - ITEM-0725 SA-10(7) Security and Privacy Representatives
      - REQ-1162 Require [Assignment: organization-defined security and privacy representatives] to be included in the [Assignment: organization-defined configuration change management and control process].
  - ITEM-0726 SA-11 Developer Testing and Evaluation
    Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0248], [ITEM-0658], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0736], [ITEM-0749], [ITEM-0906], [ITEM-1020], [ITEM-1023], [ITEM-1025].
    - REQ-1163 (a) Develop and implement a plan for ongoing security and privacy control assessments; #moderate #high #privacy
    - REQ-1164 (b) Perform [Selection (one or more): unit; integration; system; regression] testing/evaluation [Assignment: organization-defined frequency to conduct] at [Assignment: organization-defined depth and coverage]; #moderate #high #privacy
    - REQ-1165 (c) Produce evidence of the execution of the assessment plan and the results of the testing and evaluation; #moderate #high #privacy
    - REQ-1166 (d) Implement a verifiable flaw remediation process; and #moderate #high #privacy
    - REQ-1167 (e) Correct flaws identified during testing and evaluation. #moderate #high #privacy
    - ITEM-0727 SA-11(1) Static Code Analysis
      - REQ-1168 Require the developer of the system, system component, or system service to employ static code analysis tools to identify common flaws and document the results of the analysis.
    - ITEM-0728 SA-11(2) Threat Modeling and Vulnerability Analyses
      Related controls: [ITEM-0571], [ITEM-0636], [ITEM-0641].
      - REQ-1169 (a) Uses the following contextual information: [Assignment: organization-defined information];
      - REQ-1170 (b) Employs the following tools and methods: [Assignment: organization-defined tools and methods];
      - REQ-1171 (c) Conducts the modeling and analyses at the following level of rigor: [Assignment: organization-defined breadth and depth of modeling and analyses] ; and
      - REQ-1172 (d) Produces evidence that meets the following acceptance criteria: [Assignment: organization-defined acceptance criteria].
    - ITEM-0729 SA-11(3) Independent Verification of Assessment Plans and Evidence
      Related controls: [ITEM-0142], [ITEM-0641].
      - REQ-1173 (a) Require an independent agent satisfying [Assignment: organization-defined independence criteria] to verify the correct implementation of the developer security and privacy assessment plans and the evidence produced during testing and evaluation; and
      - REQ-1174 (b) Verify that the independent agent is provided with sufficient information to complete the verification process or granted the authority to obtain such information.
    - ITEM-0730 SA-11(4) Manual Code Reviews
      - REQ-1175 Require the developer of the system, system component, or system service to perform a manual code review of [Assignment: organization-defined specific code] using the following processes, procedures, and/or techniques: [Assignment: organization-defined processes, procedures, and/or techniques].
    - ITEM-0731 SA-11(5) Penetration Testing
      Related controls: [ITEM-0226], [ITEM-0570], [ITEM-0583], [ITEM-0612], [ITEM-0658], [ITEM-0906], [ITEM-0944].
      - REQ-1176 (a) At the following level of rigor: [Assignment: organization-defined breadth and depth of testing] ; and
      - REQ-1177 (b) Under the following constraints: [Assignment: organization-defined constraints].
    - ITEM-0732 SA-11(6) Attack Surface Reviews
      Related controls: [ITEM-0736].
      - REQ-1178 Require the developer of the system, system component, or system service to perform attack surface reviews.
    - ITEM-0733 SA-11(7) Verify Scope of Testing and Evaluation
      Related controls: [ITEM-0736].
      - REQ-1179 Require the developer of the system, system component, or system service to verify that the scope of testing and evaluation provides complete coverage of the required controls at the following level of rigor: [Assignment: organization-defined breadth and depth of testing and evaluation].
    - ITEM-0734 SA-11(8) Dynamic Code Analysis
      - REQ-1180 Require the developer of the system, system component, or system service to employ dynamic code analysis tools to identify common flaws and document the results of the analysis.
    - ITEM-0735 SA-11(9) Interactive Application Security Testing
      - REQ-1181 Require the developer of the system, system component, or system service to employ interactive application security testing tools to identify flaws and document the results.
  - ITEM-0736 SA-15 Development Process, Standards, and Tools
    Related controls: [ITEM-0464], [ITEM-0658], [ITEM-0662], [ITEM-0675], [ITEM-0718], [ITEM-0726], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1023], [ITEM-1027].
    - REQ-1182 (a) Require the developer of the system, system component, or system service to follow a documented development process that: #moderate #high
      (a)(1) Explicitly addresses security and privacy requirements;
      (a)(2) Identifies the standards and tools used in the development process;
      (a)(3) Documents the specific tool options and tool configurations used in the development process; and
      (a)(4) Documents, manages, and ensures the integrity of changes to the process and/or tools used in development; and
    - REQ-1183 (b) Review the development process, standards, tools, tool options, and tool configurations [Assignment: organization-defined frequency] to determine if the process, standards, tools, tool options and tool configurations selected and employed can satisfy the following security and privacy requirements: [Assignment: organization-defined security and privacy requirements]. #moderate #high
    - ITEM-0737 SA-15(1) Quality Metrics
      - REQ-1184 (a) Define quality metrics at the beginning of the development process; and
      - REQ-1185 (b) Provide evidence of meeting the quality metrics [Selection (one or more): [Assignment: organization-defined frequency] ; [Assignment: organization-defined program review] ; upon delivery].
    - ITEM-0738 SA-15(2) Security and Privacy Tracking Tools
      Related controls: [ITEM-0726].
      - REQ-1186 Require the developer of the system, system component, or system service to select and employ security and privacy tracking tools for use during the development process.
    - ITEM-0739 SA-15(3) Criticality Analysis
      Related controls: [ITEM-0653].
      - REQ-1187 (a) At the following decision points in the system development life cycle: [Assignment: organization-defined decision points] ; and #moderate #high
      - REQ-1188 (b) At the following level of rigor: [Assignment: organization-defined breadth and depth of criticality analysis]. #moderate #high
    - ITEM-0740 SA-15(5) Attack Surface Reduction
      Related controls: [ITEM-0062], [ITEM-0259], [ITEM-0636], [ITEM-0726].
      - REQ-1189 Require the developer of the system, system component, or system service to reduce attack surfaces to [Assignment: organization-defined thresholds].
    - ITEM-0741 SA-15(6) Continuous Improvement
      - REQ-1190 Require the developer of the system, system component, or system service to implement an explicit process to continuously improve the development process.
    - ITEM-0742 SA-15(7) Automated Vulnerability Analysis
      Related controls: [ITEM-0641], [ITEM-0726].
      - REQ-1191 (a) Perform an automated vulnerability analysis using [Assignment: organization-defined tools];
      - REQ-1192 (b) Determine the exploitation potential for discovered vulnerabilities;
      - REQ-1193 (c) Determine potential risk mitigations for delivered vulnerabilities; and
      - REQ-1194 (d) Deliver the outputs of the tools and results of the analysis to [Assignment: organization-defined personnel or roles].
    - ITEM-0743 SA-15(8) Reuse of Threat and Vulnerability Information
      - REQ-1195 Require the developer of the system, system component, or system service to use threat modeling and vulnerability analyses from similar systems, components, or services to inform the current development process.
    - ITEM-0744 SA-15(10) Incident Response Plan
      Related controls: [ITEM-0434].
      - REQ-1196 Require the developer of the system, system component, or system service to provide, implement, and test an incident response plan.
    - ITEM-0745 SA-15(11) Archive System or Component
      Related controls: [ITEM-0234].
      - REQ-1197 Require the developer of the system or system component to archive the system or component to be released or delivered together with the corresponding evidence supporting the final security and privacy review.
    - ITEM-0746 SA-15(12) Minimize Personally Identifiable Information
      Related controls: [ITEM-0583], [ITEM-0658], [ITEM-0675].
      - REQ-1198 Require the developer of the system or system component to minimize the use of personally identifiable information in development and test environments.
    - ITEM-0747 SA-15(13) Logging Syntax
      Related controls: [ITEM-0151], [ITEM-0152], [ITEM-0409], [ITEM-0434], [ITEM-0674].
      - REQ-1199 Require the developer of the system, system component, or system service to use [Assignment: organization-defined secure logging format(s)] to log [Assignment: organization-defined events types to log] at [Assignment: organization-defined level of detail to log].
  - ITEM-0748 SA-16 Developer-provided Training
    Related controls: [ITEM-0135], [ITEM-0142], [ITEM-0496], [ITEM-0662], [ITEM-0674].
    - REQ-1200 Require the developer of the system, system component, or system service to provide the following training on the correct use and operation of the implemented security and privacy functions, controls, and/or mechanisms: [Assignment: organization-defined training]. #high
  - ITEM-0749 SA-17 Developer Security and Privacy Architecture and Design
    Related controls: [ITEM-0544], [ITEM-0548], [ITEM-0562], [ITEM-0658], [ITEM-0662], [ITEM-0675], [ITEM-0782].
    - REQ-1201 (a) Is consistent with the organization’s security and privacy architecture that is an integral part the organization’s enterprise architecture; #high
    - REQ-1202 (b) Accurately and completely describes the required security and privacy functionality, and the allocation of controls among physical and logical components; and #high
    - REQ-1203 (c) Expresses how individual security and privacy functions, mechanisms, and services work together to provide required security and privacy capabilities and a unified approach to protection. #high
    - ITEM-0750 SA-17(1) Formal Policy Model
      Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0132].
      - REQ-1204 (a) Produce, as an integral part of the development process, a formal policy model describing the [Assignment: organization-defined elements of organizational security and privacy policy] to be enforced; and
      - REQ-1205 (b) Prove that the formal policy model is internally consistent and sufficient to enforce the defined elements of the organizational security and privacy policy when implemented.
    - ITEM-0751 SA-17(2) Security-relevant Components
      Related controls: [ITEM-0132], [ITEM-0674].
      - REQ-1206 (a) Define security-relevant hardware, software, and firmware; and
      - REQ-1207 (b) Provide a rationale that the definition for security-relevant hardware, software, and firmware is complete.
    - ITEM-0752 SA-17(3) Formal Correspondence
      Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0132], [ITEM-0662], [ITEM-0674].
      - REQ-1208 (a) Produce, as an integral part of the development process, a formal top-level specification that specifies the interfaces to security-relevant hardware, software, and firmware in terms of exceptions, error messages, and effects;
      - REQ-1209 (b) Show via proof to the extent feasible with additional informal demonstration as necessary, that the formal top-level specification is consistent with the formal policy model;
      - REQ-1210 (c) Show via informal demonstration, that the formal top-level specification completely covers the interfaces to security-relevant hardware, software, and firmware;
      - REQ-1211 (d) Show that the formal top-level specification is an accurate description of the implemented security-relevant hardware, software, and firmware; and
      - REQ-1212 (e) Describe the security-relevant hardware, software, and firmware mechanisms not addressed in the formal top-level specification but strictly internal to the security-relevant hardware, software, and firmware.
    - ITEM-0753 SA-17(4) Informal Correspondence
      Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0132], [ITEM-0662], [ITEM-0674].
      - REQ-1213 (a) Produce, as an integral part of the development process, an informal descriptive top-level specification that specifies the interfaces to security-relevant hardware, software, and firmware in terms of exceptions, error messages, and effects;
      - REQ-1214 (b) Show via [Selection: informal demonstration, convincing argument with formal methods as feasible] that the descriptive top-level specification is consistent with the formal policy model;
      - REQ-1215 (c) Show via informal demonstration, that the descriptive top-level specification completely covers the interfaces to security-relevant hardware, software, and firmware;
      - REQ-1216 (d) Show that the descriptive top-level specification is an accurate description of the interfaces to security-relevant hardware, software, and firmware; and
      - REQ-1217 (e) Describe the security-relevant hardware, software, and firmware mechanisms not addressed in the descriptive top-level specification but strictly internal to the security-relevant hardware, software, and firmware.
    - ITEM-0754 SA-17(5) Conceptually Simple Design
      Related controls: [ITEM-0132], [ITEM-0675], [ITEM-0769].
      - REQ-1218 (a) Design and structure the security-relevant hardware, software, and firmware to use a complete, conceptually simple protection mechanism with precisely defined semantics; and
      - REQ-1219 (b) Internally structure the security-relevant hardware, software, and firmware with specific regard for this mechanism.
    - ITEM-0755 SA-17(6) Structure for Testing
      Related controls: [ITEM-0674], [ITEM-0726].
      - REQ-1220 Require the developer of the system, system component, or system service to structure security-relevant hardware, software, and firmware to facilitate testing.
    - ITEM-0756 SA-17(7) Structure for Least Privilege
      Related controls: [ITEM-0061], [ITEM-0062], [ITEM-0675].
      - REQ-1221 Require the developer of the system, system component, or system service to structure security-relevant hardware, software, and firmware to facilitate controlling access with least privilege.
    - ITEM-0757 SA-17(8) Orchestration
      - REQ-1222 Design [Assignment: organization-defined critical systems] with coordinated behavior to implement the following capabilities: [Assignment: organization-defined capabilities].
    - ITEM-0758 SA-17(9) Design Diversity
      - REQ-1223 Use different designs for [Assignment: organization-defined critical systems] to satisfy a common set of requirements or to provide equivalent functionality.
  - ITEM-0759 SA-20 Customized Development of Critical Components
    Related controls: [ITEM-0291], [ITEM-0653], [ITEM-0675].
    - REQ-1224 Reimplement or custom develop the following critical system components: [Assignment: organization-defined critical system].
  - ITEM-0760 SA-21 Developer Screening
    Related controls: [ITEM-0594], [ITEM-0595], [ITEM-0604], [ITEM-0607], [ITEM-0662], [ITEM-1023].
    - REQ-1225 (a) Has appropriate access authorizations as determined by assigned [Assignment: organization-defined official government duties] ; and #high
    - REQ-1226 (b) Satisfies the following additional personnel screening criteria: [Assignment: organization-defined additional personnel screening criteria]. #high
  - ITEM-0761 SA-22 Unsupported System Components
    Related controls: [ITEM-0544], [ITEM-0658].
    - REQ-1227 (a) Replace system components when support for the components is no longer available from the developer, vendor, or manufacturer; or #low #moderate #high
    - REQ-1228 (b) Provide the following options for alternative sources for continued support for unsupported components [Selection (one or more): in-house support; [Assignment: organization-defined support from external providers] ]. #low #moderate #high
  - ITEM-0762 SA-23 Specialization
    Related controls: [ITEM-0653], [ITEM-0675].
    - REQ-1229 Employ [Selection (one or more): design modification; augmentation; reconfiguration] on [Assignment: organization-defined systems or system components] supporting mission essential services or functions to increase the trustworthiness in those systems or components.
  - ITEM-0763 SA-24 Design For Cyber Resiliency
    Related controls: [ITEM-0220], [ITEM-0291], [ITEM-0302], [ITEM-0324], [ITEM-0332], [ITEM-0336], [ITEM-0337], [ITEM-0338], [ITEM-0386], [ITEM-0409], [ITEM-0425], [ITEM-0519], [ITEM-0534], [ITEM-0548], [ITEM-0562], [ITEM-0572], [ITEM-0588], [ITEM-0590], [ITEM-0636], [ITEM-0641], [ITEM-0653], [ITEM-0654], [ITEM-0658], [ITEM-0675], [ITEM-0709], [ITEM-0749], [ITEM-0769], [ITEM-0777], [ITEM-0782], [ITEM-0815], [ITEM-0816], [ITEM-0855], [ITEM-0857], [ITEM-0868], [ITEM-0871], [ITEM-0872], [ITEM-0875], [ITEM-0878], [ITEM-0893], [ITEM-0898], [ITEM-0899], [ITEM-0901], [ITEM-0902], [ITEM-0903], [ITEM-0913], [ITEM-0918], [ITEM-0944], [ITEM-0947], [ITEM-0964], [ITEM-0981], [ITEM-0985], [ITEM-0986], [ITEM-0994], [ITEM-1003], [ITEM-1004], [ITEM-1005], [ITEM-1006], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1023], [ITEM-1025], [ITEM-1027], [ITEM-1029], [ITEM-1030].
    - REQ-1230 (a) Design organizational systems, system components, or system services to achieve cyber resiliency by:
      (a)(1) Defining the following cyber resiliency goals: [Assignment: organization-defined cyber resiliency goals].
      (a)(2) Defining the following cyber resiliency objectives: [Assignment: organization-defined cyber resiliency objectives].
      (a)(3) Defining the following cyber resiliency techniques: [Assignment: organization-defined cyber resiliency techniques].
      (a)(4) Defining the following cyber resiliency implementation approaches: [Assignment: organization-defined cyber resiliency implementation approaches].
      (a)(5) Defining the following cyber resiliency design principles: [Assignment: organization-defined cyber resiliency design principles].
    - REQ-1231 (b) Implement the selected cyber resiliency goals, objectives, techniques, implementation approaches, and design principles as part of an organizational risk management process or systems security engineering process.
- ITEM-0764 System and Communications Protection
  The system and communications protection family (SC) of NIST SP 800-53 Revision 5.
  - ITEM-0765 SC-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0675], [ITEM-0972].
    - REQ-1232 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business-process-level; system-level] system and communications protection policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the system and communications protection policy and the associated system and communications protection controls;
    - REQ-1233 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the system and communications protection policy and procedures; and #low #moderate #high
    - REQ-1234 (c) Review and update the current system and communications protection: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0766 SC-2 Separation of System and User Functionality
    Related controls: [ITEM-0062], [ITEM-0662], [ITEM-0675], [ITEM-0769], [ITEM-0782], [ITEM-0842], [ITEM-0866], [ITEM-0878].
    - REQ-1235 Separate user functionality, including user interface services, from system management functionality. #moderate #high
    - ITEM-0767 SC-2(1) Interfaces for Non-privileged Users
      Related controls: [ITEM-0016].
      - REQ-1236 Prevent the presentation of system management functionality at interfaces to non-privileged users.
    - ITEM-0768 SC-2(2) Disassociability
      - REQ-1237 Store state information from applications and software separately.
  - ITEM-0769 SC-3 Security Function Isolation
    Related controls: [ITEM-0016], [ITEM-0062], [ITEM-0132], [ITEM-0234], [ITEM-0248], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0736], [ITEM-0749], [ITEM-0766], [ITEM-0782], [ITEM-0866], [ITEM-0878], [ITEM-0986].
    - REQ-1238 Isolate security functions from nonsecurity functions. #high
    - ITEM-0770 SC-3(1) Hardware Separation
      - REQ-1239 Employ hardware separation mechanisms to implement security function isolation.
    - ITEM-0771 SC-3(2) Access and Flow Control Functions
      - REQ-1240 Isolate security functions enforcing access and information flow control from nonsecurity functions and from other security functions.
    - ITEM-0772 SC-3(3) Minimize Nonsecurity Functionality
      - REQ-1241 Minimize the number of nonsecurity functions included within the isolation boundary containing security functions.
    - ITEM-0773 SC-3(4) Module Coupling and Cohesiveness
      - REQ-1242 Implement security functions as largely independent modules that maximize internal cohesiveness within modules and minimize coupling between modules.
    - ITEM-0774 SC-3(5) Layered Structures
      - REQ-1243 Implement security functions as a layered structure minimizing interactions between layers of the design and avoiding any dependence by lower layers on the functionality or correctness of higher layers.
  - ITEM-0775 SC-4 Information in Shared System Resources
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0675].
    - REQ-1244 Prevent unauthorized and unintended information transfer via shared system resources. #moderate #high
    - ITEM-0776 SC-4(2) Multilevel or Periods Processing
      - REQ-1245 Prevent unauthorized information transfer via shared resources in accordance with [Assignment: organization-defined procedures] when system processing explicitly switches between different information classification levels or security categories.
  - ITEM-0777 SC-5 Denial-of-service Protection
    Related controls: [ITEM-0291], [ITEM-0409], [ITEM-0781], [ITEM-0782], [ITEM-0881].
    - REQ-1246 (a) [Selection: protect against; limit] the effects of the following types of denial-of-service events: [Assignment: organization-defined types of denial-of-service events] ; and #low #moderate #high
    - REQ-1247 (b) Employ the following controls to achieve the denial-of-service objective: [Assignment: organization-defined controls by type of denial-of-service event]. #low #moderate #high
    - ITEM-0778 SC-5(1) Restrict Ability to Attack Other Systems
      - REQ-1248 Restrict the ability of individuals to launch the following denial-of-service attacks against other systems: [Assignment: organization-defined denial-of-service attacks].
    - ITEM-0779 SC-5(2) Capacity, Bandwidth, and Redundancy
      - REQ-1249 Manage capacity, bandwidth, or other redundancy to limit the effects of information flooding denial-of-service attacks.
    - ITEM-0780 SC-5(3) Detection and Monitoring
      Related controls: [ITEM-0220], [ITEM-0918].
      - REQ-1250 (a) Employ the following monitoring tools to detect indicators of denial-of-service attacks against, or launched from, the system: [Assignment: organization-defined monitoring tools] ; and
      - REQ-1251 (b) Monitor the following system resources to determine if sufficient resources exist to prevent effective denial-of-service attacks: [Assignment: organization-defined system resources].
  - ITEM-0781 SC-6 Resource Availability
    Related controls: [ITEM-0777].
    - REQ-1252 Protect the availability of resources by allocating [Assignment: organization-defined resources] by [Selection (one or more): priority; quota; [Assignment: organization-defined controls] ].
  - ITEM-0782 SC-7 Boundary Protection
    Related controls: [ITEM-0030], [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0118], [ITEM-0195], [ITEM-0212], [ITEM-0234], [ITEM-0248], [ITEM-0259], [ITEM-0280], [ITEM-0318], [ITEM-0332], [ITEM-0409], [ITEM-0451], [ITEM-0496], [ITEM-0548], [ITEM-0568], [ITEM-0675], [ITEM-0749], [ITEM-0777], [ITEM-0849], [ITEM-0866], [ITEM-0871], [ITEM-0892].
    - REQ-1253 (a) Monitor and control communications at the external managed interfaces to the system and at key internal managed interfaces within the system; #low #moderate #high
    - REQ-1254 (b) Implement subnetworks for publicly accessible system components that are [Selection: physically; logically] separated from internal organizational networks; and #low #moderate #high
    - REQ-1255 (c) Connect to external networks or systems only through managed interfaces consisting of boundary protection devices arranged in accordance with an organizational security and privacy architecture. #low #moderate #high
    - ITEM-0783 SC-7(3) Access Points
      - REQ-1256 Limit the number of external network connections to the system. #moderate #high
    - ITEM-0784 SC-7(4) External Telecommunications Services
      Related controls: [ITEM-0016], [ITEM-0809], [ITEM-0839], [ITEM-0841], [ITEM-0842].
      - REQ-1257 (a) Implement a managed interface for each external telecommunication service; #moderate #high
      - REQ-1258 (b) Establish a traffic flow policy for each managed interface; #moderate #high
      - REQ-1259 (c) Protect the confidentiality and integrity of the information being transmitted across each interface; #moderate #high
      - REQ-1260 (d) Document each exception to the traffic flow policy with a supporting mission or business need and duration of that need; #moderate #high
      - REQ-1261 (e) Review exceptions to the traffic flow policy [Assignment: organization-defined frequency] and remove exceptions that are no longer supported by an explicit mission or business need; #moderate #high
      - REQ-1262 (f) Prevent unauthorized exchange of control plane traffic with external networks; #moderate #high
      - REQ-1263 (g) Publish information to enable remote networks to detect unauthorized control plane traffic from internal networks; and #moderate #high
      - REQ-1264 (h) Filter unauthorized control plane traffic from external networks. #moderate #high
    - ITEM-0785 SC-7(5) Deny by Default — Allow by Exception
      - REQ-1265 Deny network communications traffic by default and allow network communications traffic by exception [Selection (one or more): at managed interfaces; for [Assignment: organization-defined systems] ]. #moderate #high
    - ITEM-0786 SC-7(7) Split Tunneling for Remote Devices
      - REQ-1266 Prevent split tunneling for remote devices connecting to organizational systems unless the split tunnel is securely provisioned using [Assignment: organization-defined safeguards]. #moderate #high
    - ITEM-0787 SC-7(8) Route Traffic to Authenticated Proxy Servers
      Related controls: [ITEM-0016].
      - REQ-1267 Route [Assignment: organization-defined internal communications traffic] to [Assignment: organization-defined external networks] through authenticated proxy servers at managed interfaces. #moderate #high
    - ITEM-0788 SC-7(9) Restrict Threatening Outgoing Communications Traffic
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0777], [ITEM-0877], [ITEM-0893], [ITEM-0913], [ITEM-0918].
      - REQ-1268 (a) Detect and deny outgoing communications traffic posing a threat to external systems; and
      - REQ-1269 (b) Audit the identity of internal users associated with denied communications.
    - ITEM-0789 SC-7(10) Prevent Exfiltration
      Related controls: [ITEM-0003], [ITEM-0226], [ITEM-0913].
      - REQ-1270 (a) Prevent the exfiltration of information; and
      - REQ-1271 (b) Conduct exfiltration tests [Assignment: organization-defined frequency].
    - ITEM-0790 SC-7(11) Restrict Incoming Communications Traffic
      Related controls: [ITEM-0016].
      - REQ-1272 Only allow incoming communications from [Assignment: organization-defined authorized sources] to be routed to [Assignment: organization-defined authorized destinations].
    - ITEM-0791 SC-7(12) Host-based Protection
      - REQ-1273 Implement [Assignment: organization-defined host-based boundary protection mechanisms] at [Assignment: organization-defined system components].
    - ITEM-0792 SC-7(13) Isolation of Security Tools, Mechanisms, and Support Components
      Related controls: [ITEM-0766], [ITEM-0769].
      - REQ-1274 Isolate [Assignment: organization-defined information security tools, mechanisms, and support components] from other internal system components by implementing physically separate subnetworks with managed interfaces to other components of the system.
    - ITEM-0793 SC-7(14) Protect Against Unauthorized Physical Connections
      Related controls: [ITEM-0504], [ITEM-0536].
      - REQ-1275 Protect against unauthorized physical connections at [Assignment: organization-defined managed interfaces].
    - ITEM-0794 SC-7(15) Networked Privileged Accesses
      Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0151], [ITEM-0918].
      - REQ-1276 Route networked, privileged accesses through a dedicated, managed interface for purposes of access control and auditing.
    - ITEM-0795 SC-7(16) Prevent Discovery of System Components
      - REQ-1277 Prevent the discovery of specific system components that represent a managed interface.
    - ITEM-0796 SC-7(17) Automated Enforcement of Protocol Formats
      Related controls: [ITEM-0775].
      - REQ-1278 Enforce adherence to protocol formats.
    - ITEM-0797 SC-7(18) Fail Secure
      Related controls: [ITEM-0291], [ITEM-0337], [ITEM-0847].
      - REQ-1279 Prevent systems from entering unsecure states in the event of an operational failure of a boundary protection device. #high
    - ITEM-0798 SC-7(19) Block Communication from Non-organizationally Configured Hosts
      - REQ-1280 Block inbound and outbound communications traffic between [Assignment: organization-defined communication clients] that are independently configured by end users and external service providers.
    - ITEM-0799 SC-7(20) Dynamic Isolation and Segregation
      - REQ-1281 Provide the capability to dynamically isolate [Assignment: organization-defined system components] from other system components.
    - ITEM-0800 SC-7(21) Isolation of System Components
      Related controls: [ITEM-0230].
      - REQ-1282 Employ boundary protection mechanisms to isolate [Assignment: organization-defined system components] supporting [Assignment: organization-defined missions and/or business functions]. #high
    - ITEM-0801 SC-7(22) Separate Subnets for Connecting to Different Security Domains
      - REQ-1283 Implement separate network addresses to connect to systems in different security domains.
    - ITEM-0802 SC-7(23) Disable Sender Feedback on Protocol Validation Failure
      - REQ-1284 Disable feedback to senders on protocol format validation failure.
    - ITEM-0803 SC-7(24) Personally Identifiable Information
      Related controls: [ITEM-0612], [ITEM-0985].
      - REQ-1285 (a) Apply the following processing rules to data elements of personally identifiable information: [Assignment: organization-defined processing rules]; #privacy
      - REQ-1286 (b) Monitor for permitted processing at the external interfaces to the system and at key internal boundaries within the system; #privacy
      - REQ-1287 (c) Document each processing exception; and #privacy
      - REQ-1288 (d) Review and remove exceptions that are no longer supported. #privacy
    - ITEM-0804 SC-7(25) Unclassified National Security System Connections
      - REQ-1289 Prohibit the direct connection of [Assignment: organization-defined unclassified national security system] to an external network without the use of [Assignment: organization-defined boundary protection device].
    - ITEM-0805 SC-7(26) Classified National Security System Connections
      - REQ-1290 Prohibit the direct connection of a classified national security system to an external network without the use of [Assignment: organization-defined boundary protection device].
    - ITEM-0806 SC-7(27) Unclassified Non-national Security System Connections
      - REQ-1291 Prohibit the direct connection of [Assignment: organization-defined unclassified, non-national security system] to an external network without the use of [Assignment: organization-defined boundary protection device].
    - ITEM-0807 SC-7(28) Connections to Public Networks
      - REQ-1292 Prohibit the direct connection of [Assignment: organization-defined system] to a public network.
    - ITEM-0808 SC-7(29) Separate Subnets to Isolate Functions
      - REQ-1293 Implement [Selection: physically; logically] separate subnetworks to isolate the following critical system components and functions: [Assignment: organization-defined critical system components and functions].
  - ITEM-0809 SC-8 Transmission Confidentiality and Integrity
    Related controls: [ITEM-0102], [ITEM-0110], [ITEM-0183], [ITEM-0350], [ITEM-0379], [ITEM-0385], [ITEM-0451], [ITEM-0504], [ITEM-0662], [ITEM-0675], [ITEM-0782], [ITEM-0828], [ITEM-0839], [ITEM-0843], [ITEM-0851].
    - REQ-1294 Protect the [Selection (one or more): confidentiality; integrity] of transmitted information. #moderate #high
    - ITEM-0810 SC-8(1) Cryptographic Protection
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1295 Implement cryptographic mechanisms to [Selection (one or more): prevent unauthorized disclosure of information; detect changes to information] during transmission. #moderate #high
    - ITEM-0811 SC-8(2) Pre- and Post-transmission Handling
      - REQ-1296 Maintain the [Selection (one or more): confidentiality; integrity] of information during preparation for transmission and during reception.
    - ITEM-0812 SC-8(3) Cryptographic Protection for Message Externals
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1297 Implement cryptographic mechanisms to protect message externals unless otherwise protected by [Assignment: organization-defined alternative physical controls].
    - ITEM-0813 SC-8(4) Conceal or Randomize Communications
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1298 Implement cryptographic mechanisms to conceal or randomize communication patterns unless otherwise protected by [Assignment: organization-defined alternative physical controls].
    - ITEM-0814 SC-8(5) Protected Distribution System
      - REQ-1299 Implement [Assignment: organization-defined protected distribution system] to [Selection (one or more): prevent unauthorized disclosure of information; detect changes to information] during transmission.
  - ITEM-0815 SC-10 Network Disconnect
    Related controls: [ITEM-0102], [ITEM-0843].
    - REQ-1300 Terminate the network connection associated with a communications session at the end of the session or after [Assignment: organization-defined time period] of inactivity. #moderate #high
  - ITEM-0816 SC-11 Trusted Path
    Related controls: [ITEM-0091], [ITEM-0132], [ITEM-0818], [ITEM-0843].
    - REQ-1301 (a) Provide a [Selection: physically; logically] isolated trusted communications path for communications between the user and the trusted components of the system; and
    - REQ-1302 (b) Permit users to invoke the trusted communications path for communications between the user and the following security functions of the system, including at a minimum, authentication and re-authentication: [Assignment: organization-defined security functions].
    - ITEM-0817 SC-11(1) Irrefutable Communications Path
      - REQ-1303 (a) Provide a trusted communications path that is irrefutably distinguishable from other communications paths; and
      - REQ-1304 (b) Initiate the trusted communications path for communications between the [Assignment: organization-defined security functions] of the system and the user.
  - ITEM-0818 SC-12 Cryptographic Key Establishment and Management
    Related controls: [ITEM-0102], [ITEM-0175], [ITEM-0183], [ITEM-0239], [ITEM-0350], [ITEM-0378], [ITEM-0395], [ITEM-0662], [ITEM-0675], [ITEM-0709], [ITEM-0809], [ITEM-0816], [ITEM-0823], [ITEM-0832], [ITEM-0839], [ITEM-0875], [ITEM-0881], [ITEM-0913], [ITEM-0947].
    - REQ-1305 Establish and manage cryptographic keys when cryptography is employed within the system in accordance with the following key management requirements: [Assignment: organization-defined requirements]. #low #moderate #high
    - ITEM-0819 SC-12(1) Availability
      - REQ-1306 Maintain availability of information in the event of the loss of cryptographic keys by users. #high
    - ITEM-0820 SC-12(2) Symmetric Keys
      - REQ-1307 Produce, control, and distribute symmetric cryptographic keys using [Selection: NIST FIPS-validated; NSA-approved] key management technology and processes.
    - ITEM-0821 SC-12(3) Asymmetric Keys
      - REQ-1308 Produce, control, and distribute asymmetric cryptographic keys using [Selection: NSA-approved key management technology and processes; prepositioned keying material; DoD-approved or DoD-issued Medium Assurance PKI certificates; DoD-approved or DoD-issued Medium Hardware Assurance PKI certificates and hardware security tokens that protect the user’s private key; certificates issued in accordance with organization-defined requirements].
    - ITEM-0822 SC-12(6) Physical Control of Keys
      - REQ-1309 Maintain physical control of cryptographic keys when stored information is encrypted by external service providers.
  - ITEM-0823 SC-13 Cryptographic Protection
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0073], [ITEM-0102], [ITEM-0110], [ITEM-0115], [ITEM-0175], [ITEM-0183], [ITEM-0282], [ITEM-0324], [ITEM-0350], [ITEM-0361], [ITEM-0378], [ITEM-0395], [ITEM-0451], [ITEM-0471], [ITEM-0473], [ITEM-0475], [ITEM-0662], [ITEM-0675], [ITEM-0709], [ITEM-0809], [ITEM-0818], [ITEM-0839], [ITEM-0843], [ITEM-0851], [ITEM-0881], [ITEM-0913], [ITEM-0947].
    - REQ-1310 (a) Determine the [Assignment: organization-defined cryptographic uses] ; and #low #moderate #high
    - REQ-1311 (b) Implement the following types of cryptography required for each specified cryptographic use: [Assignment: organization-defined types of cryptography]. #low #moderate #high
  - ITEM-0824 SC-15 Collaborative Computing Devices and Applications
    Related controls: [ITEM-0124], [ITEM-0887].
    - REQ-1312 (a) Prohibit remote activation of collaborative computing devices and applications with the following exceptions: [Assignment: organization-defined exceptions where remote activation is to be allowed] ; and #low #moderate #high
    - REQ-1313 (b) Provide an explicit indication of use to users physically present at the devices. #low #moderate #high
    - ITEM-0825 SC-15(1) Physical or Logical Disconnect
      - REQ-1314 Provide [Selection (one or more): physical; logical] disconnect of collaborative computing devices in a manner that supports ease of use.
    - ITEM-0826 SC-15(3) Disabling and Removal in Secure Work Areas
      - REQ-1315 Disable or remove collaborative computing devices and applications from [Assignment: organization-defined systems or system components] in [Assignment: organization-defined secure work areas].
    - ITEM-0827 SC-15(4) Explicitly Indicate Current Participants
      - REQ-1316 Provide an explicit indication of current participants in [Assignment: organization-defined online meetings and teleconferences].
  - ITEM-0828 SC-16 Transmission of Security and Privacy Attributes
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0091].
    - REQ-1317 Associate [Assignment: organization-defined security and privacy attributes] with information exchanged between systems and between system components.
    - ITEM-0829 SC-16(1) Integrity Verification
      Related controls: [ITEM-0183], [ITEM-0809].
      - REQ-1318 Verify the integrity of transmitted security and privacy attributes.
    - ITEM-0830 SC-16(2) Anti-spoofing Mechanisms
      Related controls: [ITEM-0913], [ITEM-0918], [ITEM-0947].
      - REQ-1319 Implement anti-spoofing mechanisms to prevent adversaries from falsifying the security attributes indicating the successful application of the security process.
    - ITEM-0831 SC-16(3) Cryptographic Binding
      Related controls: [ITEM-0091], [ITEM-0818], [ITEM-0823].
      - REQ-1320 Implement [Assignment: organization-defined mechanisms or techniques] to bind security and privacy attributes to transmitted information.
  - ITEM-0832 SC-17 Public Key Infrastructure Certificates
    Related controls: [ITEM-0183], [ITEM-0361], [ITEM-0818].
    - REQ-1321 (a) Issue public key certificates under an [Assignment: organization-defined certificate policy] or obtain public key certificates from an approved service provider; and #moderate #high
    - REQ-1322 (b) Include only approved trust anchors in trust stores or certificate stores managed by the organization. #moderate #high
  - ITEM-0833 SC-18 Mobile Code
    Related controls: [ITEM-0151], [ITEM-0190], [ITEM-0234], [ITEM-0256], [ITEM-0913].
    - REQ-1323 (a) Define acceptable and unacceptable mobile code and mobile code technologies; and #moderate #high
    - REQ-1324 (b) Authorize, monitor, and control the use of mobile code within the system. #moderate #high
    - ITEM-0834 SC-18(1) Identify Unacceptable Code and Take Corrective Actions
      - REQ-1325 Identify [Assignment: organization-defined unacceptable mobile code] and take [Assignment: organization-defined corrective actions].
    - ITEM-0835 SC-18(2) Acquisition, Development, and Use
      - REQ-1326 Verify that the acquisition, development, and use of mobile code to be deployed in the system meets [Assignment: organization-defined mobile code requirements].
    - ITEM-0836 SC-18(3) Prevent Downloading and Execution
      - REQ-1327 Prevent the download and execution of [Assignment: organization-defined unacceptable mobile code].
    - ITEM-0837 SC-18(4) Prevent Automatic Execution
      - REQ-1328 Prevent the automatic execution of mobile code in [Assignment: organization-defined software applications] and enforce [Assignment: organization-defined actions] prior to executing the code.
    - ITEM-0838 SC-18(5) Allow Execution Only in Confined Environments
      Related controls: [ITEM-0893], [ITEM-0947].
      - REQ-1329 Allow execution of permitted mobile code only in confined virtual machine environments.
  - ITEM-0839 SC-20 Secure Name/Address Resolution Service (Authoritative Source)
    Related controls: [ITEM-0183], [ITEM-0809], [ITEM-0818], [ITEM-0823], [ITEM-0841], [ITEM-0842].
    - REQ-1330 (a) Provide additional data origin authentication and integrity verification artifacts along with the authoritative name resolution data the system returns in response to external name/address resolution queries; and #low #moderate #high
    - REQ-1331 (b) Provide the means to indicate the security status of child zones and (if the child supports secure resolution services) to enable verification of a chain of trust among parent and child domains, when operating as part of a distributed, hierarchical namespace. #low #moderate #high
    - ITEM-0840 SC-20(2) Data Origin and Integrity
      - REQ-1332 Provide data origin and integrity protection artifacts for internal name/address resolution queries.
  - ITEM-0841 SC-21 Secure Name/Address Resolution Service (Recursive or Caching Resolver)
    Related controls: [ITEM-0839], [ITEM-0842].
    - REQ-1333 Request and perform data origin authentication and data integrity verification on the name/address resolution responses the system receives from authoritative sources. #low #moderate #high
  - ITEM-0842 SC-22 Architecture and Provisioning for Name/Address Resolution Service
    Related controls: [ITEM-0766], [ITEM-0839], [ITEM-0841], [ITEM-0847].
    - REQ-1334 Ensure the systems that collectively provide name/address resolution service for an organization are fault-tolerant and implement internal and external role separation. #low #moderate #high
  - ITEM-0843 SC-23 Session Authenticity
    Related controls: [ITEM-0183], [ITEM-0809], [ITEM-0815], [ITEM-0816].
    - REQ-1335 Protect the authenticity of communications sessions. #moderate #high
    - ITEM-0844 SC-23(1) Invalidate Session Identifiers at Logout
      - REQ-1336 Invalidate session identifiers upon user logout or other session termination.
    - ITEM-0845 SC-23(3) Unique System-generated Session Identifiers
      Related controls: [ITEM-0083], [ITEM-0818], [ITEM-0823].
      - REQ-1337 Generate a unique session identifier for each session with [Assignment: organization-defined randomness requirements] and recognize only session identifiers that are system-generated.
    - ITEM-0846 SC-23(5) Allowed Certificate Authorities
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1338 Only allow the use of [Assignment: organization-defined certificated authorities] for verification of the establishment of protected sessions.
  - ITEM-0847 SC-24 Fail in Known State
    Related controls: [ITEM-0291], [ITEM-0302], [ITEM-0332], [ITEM-0337], [ITEM-0675], [ITEM-0782], [ITEM-0842], [ITEM-0976].
    - REQ-1339 Fail to a [Assignment: organization-defined known system state] for the following failures on the indicated components while preserving [Assignment: organization-defined system state information] in failure: [Assignment: organization-defined types of system failures on system components]. #high
  - ITEM-0848 SC-25 Thin Nodes
    Related controls: [ITEM-0857], [ITEM-0893].
    - REQ-1340 Employ minimal functionality and information storage on the following system components: [Assignment: organization-defined system components].
  - ITEM-0849 SC-26 Decoys
    Related controls: [ITEM-0641], [ITEM-0782], [ITEM-0857], [ITEM-0871], [ITEM-0893], [ITEM-0913], [ITEM-0918].
    - REQ-1341 Include components within organizational systems specifically designed to be the target of malicious attacks for detecting, deflecting, and analyzing such attacks.
  - ITEM-0850 SC-27 Platform-independent Applications
    Related controls: [ITEM-0855].
    - REQ-1342 Include within organizational systems the following platform independent applications: [Assignment: organization-defined platform-independent applications].
  - ITEM-0851 SC-28 Protection of Information at Rest
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0062], [ITEM-0115], [ITEM-0220], [ITEM-0239], [ITEM-0251], [ITEM-0256], [ITEM-0324], [ITEM-0473], [ITEM-0475], [ITEM-0496], [ITEM-0809], [ITEM-0818], [ITEM-0823], [ITEM-0868], [ITEM-0913], [ITEM-0947], [ITEM-0986].
    - REQ-1343 Protect the [Selection (one or more): confidentiality; integrity] of the following information at rest: [Assignment: organization-defined information at rest]. #moderate #high
    - ITEM-0852 SC-28(1) Cryptographic Protection
      Related controls: [ITEM-0115], [ITEM-0818], [ITEM-0823].
      - REQ-1344 Implement cryptographic mechanisms to prevent unauthorized disclosure and modification of the following information at rest on [Assignment: organization-defined system components or media]: [Assignment: organization-defined information]. #moderate #high
    - ITEM-0853 SC-28(2) Offline Storage
      - REQ-1345 Remove the following information from online storage and store offline in a secure location: [Assignment: organization-defined information].
    - ITEM-0854 SC-28(3) Cryptographic Keys
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1346 Provide protected storage for cryptographic keys [Selection: [Assignment: organization-defined safeguards] ; hardware-protected key store].
  - ITEM-0855 SC-29 Heterogeneity
    Related controls: [ITEM-0175], [ITEM-0548], [ITEM-0850], [ITEM-0857], [ITEM-1011].
    - REQ-1347 Employ a diverse set of information technologies for the following system components in the implementation of the system: [Assignment: organization-defined system components].
    - ITEM-0856 SC-29(1) Virtualization Techniques
      - REQ-1348 Employ virtualization techniques to support the deployment of a diversity of operating systems and applications that are changed [Assignment: organization-defined frequency].
  - ITEM-0857 SC-30 Concealment and Misdirection
    Related controls: [ITEM-0062], [ITEM-0848], [ITEM-0849], [ITEM-0855], [ITEM-0893], [ITEM-0981].
    - REQ-1349 Employ the following concealment and misdirection techniques for [Assignment: organization-defined systems] at [Assignment: organization-defined time periods] to confuse and mislead adversaries: [Assignment: organization-defined concealment and misdirection techniques].
    - ITEM-0858 SC-30(2) Randomness
      - REQ-1350 Employ [Assignment: organization-defined techniques] to introduce randomness into organizational operations and assets.
    - ITEM-0859 SC-30(3) Change Processing and Storage Locations
      - REQ-1351 Change the location of [Assignment: organization-defined processing and/or storage] [Selection: [Assignment: organization-defined time frequency] ; random time intervals]].
    - ITEM-0860 SC-30(4) Misleading Information
      - REQ-1352 Employ realistic, but misleading information in [Assignment: organization-defined system components] about its security state or posture.
    - ITEM-0861 SC-30(5) Concealment of System Components
      - REQ-1353 Employ the following techniques to hide or conceal [Assignment: organization-defined system components]: [Assignment: organization-defined techniques].
  - ITEM-0862 SC-31 Covert Channel Analysis
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0675], [ITEM-0971].
    - REQ-1354 (a) Perform a covert channel analysis to identify those aspects of communications within the system that are potential avenues for covert [Selection (one or more): storage; timing] channels; and
    - REQ-1355 (b) Estimate the maximum bandwidth of those channels.
    - ITEM-0863 SC-31(1) Test Covert Channels for Exploitability
      - REQ-1356 Test a subset of the identified covert channels to determine the channels that are exploitable.
    - ITEM-0864 SC-31(2) Maximum Bandwidth
      - REQ-1357 Reduce the maximum bandwidth for identified covert [Selection (one or more): storage; timing] channels to [Assignment: organization-defined values].
    - ITEM-0865 SC-31(3) Measure Bandwidth in Operational Environments
      - REQ-1358 Measure the bandwidth of [Assignment: organization-defined subset of identified covert channels] in the operational environment of the system.
  - ITEM-0866 SC-32 System Partitioning
    Related controls: [ITEM-0030], [ITEM-0062], [ITEM-0675], [ITEM-0766], [ITEM-0769], [ITEM-0782], [ITEM-0872].
    - REQ-1359 Partition the system into [Assignment: organization-defined system components] residing in separate [Selection: physical; logical] domains or environments based on [Assignment: organization-defined circumstances for the physical or logical separation of components].
    - ITEM-0867 SC-32(1) Separate Physical Domains for Privileged Functions
      - REQ-1360 Partition privileged functions into separate physical domains.
  - ITEM-0868 SC-34 Non-modifiable Executable Programs
    Related controls: [ITEM-0016], [ITEM-0947], [ITEM-0981].
    - REQ-1361 (a) The operating environment from hardware-enforced, read-only media; and
    - REQ-1362 (b) The following applications from hardware-enforced, read-only media: [Assignment: organization-defined applications].
    - ITEM-0869 SC-34(1) No Writable Storage
      Related controls: [ITEM-0115], [ITEM-0483].
      - REQ-1363 Employ [Assignment: organization-defined system components] with no writeable storage that is persistent across component restart or power on/off.
    - ITEM-0870 SC-34(2) Integrity Protection on Read-only Media
      Related controls: [ITEM-0239], [ITEM-0251], [ITEM-0278], [ITEM-0471], [ITEM-0473], [ITEM-0475], [ITEM-0851], [ITEM-0913].
      - REQ-1364 Protect the integrity of information prior to storage on read-only media and control the media after such information has been recorded onto the media.
  - ITEM-0871 SC-35 External Malicious Code Identification
    Related controls: [ITEM-0782], [ITEM-0849], [ITEM-0893], [ITEM-0913], [ITEM-0918].
    - REQ-1365 Include system components that proactively seek to identify network-based malicious code or malicious websites.
  - ITEM-0872 SC-36 Distributed Processing and Storage
    Related controls: [ITEM-0308], [ITEM-0312], [ITEM-0548], [ITEM-0866].
    - REQ-1366 Distribute the following processing and storage components across multiple [Selection: physical locations; logical domains]: [Assignment: organization-defined processing and storage components].
    - ITEM-0873 SC-36(1) Polling Techniques
      Related controls: [ITEM-0918].
      - REQ-1367 (a) Employ polling techniques to identify potential faults, errors, or compromises to the following processing and storage components: [Assignment: organization-defined distributed processing and storage components] ; and
      - REQ-1368 (b) Take the following actions in response to identified faults, errors, or compromises: [Assignment: organization-defined actions].
    - ITEM-0874 SC-36(2) Synchronization
      Related controls: [ITEM-0324].
      - REQ-1369 Synchronize the following duplicate systems or system components: [Assignment: organization-defined duplicate systems or system components].
  - ITEM-0875 SC-37 Out-of-band Channels
    Related controls: [ITEM-0003], [ITEM-0239], [ITEM-0251], [ITEM-0259], [ITEM-0341], [ITEM-0354], [ITEM-0361], [ITEM-0451], [ITEM-0818], [ITEM-0913], [ITEM-0918], [ITEM-0947].
    - REQ-1370 Employ the following out-of-band channels for the physical delivery or electronic transmission of [Assignment: organization-defined information, system components, or devices] to [Assignment: organization-defined individuals or systems]: [Assignment: organization-defined out-of-band channels].
    - ITEM-0876 SC-37(1) Ensure Delivery and Transmission
      - REQ-1371 Employ [Assignment: organization-defined controls] to ensure that only [Assignment: organization-defined individuals or systems] receive the following information, system components, or devices: [Assignment: organization-defined information, system components, or devices].
  - ITEM-0877 SC-38 Operations Security
    Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0543], [ITEM-0565], [ITEM-0568], [ITEM-0634], [ITEM-0636], [ITEM-0641], [ITEM-0782], [ITEM-1011], [ITEM-1025].
    - REQ-1372 Employ the following operations security controls to protect key organizational information throughout the system development life cycle: [Assignment: organization-defined operations security controls].
  - ITEM-0878 SC-39 Process Isolation
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0062], [ITEM-0132], [ITEM-0675], [ITEM-0766], [ITEM-0769], [ITEM-0986].
    - REQ-1373 Maintain a separate execution domain for each executing system process. #low #moderate #high
    - ITEM-0879 SC-39(1) Hardware Separation
      - REQ-1374 Implement hardware separation mechanisms to facilitate process isolation.
    - ITEM-0880 SC-39(2) Separate Execution Domain Per Thread
      - REQ-1375 Maintain a separate execution domain for each thread in [Assignment: organization-defined multi-threaded processing].
  - ITEM-0881 SC-40 Wireless Link Protection
    Related controls: [ITEM-0110], [ITEM-0777].
    - REQ-1376 Protect external and internal [Assignment: organization-defined wireless links] from the following signal parameter attacks: [Assignment: organization-defined types of signal parameter attacks or references to sources for such attacks].
    - ITEM-0882 SC-40(1) Electromagnetic Interference
      Related controls: [ITEM-0539], [ITEM-0818], [ITEM-0823].
      - REQ-1377 Implement cryptographic mechanisms that achieve [Assignment: organization-defined level of protection] against the effects of intentional electromagnetic interference.
    - ITEM-0883 SC-40(2) Reduce Detection Potential
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1378 Implement cryptographic mechanisms to reduce the detection potential of wireless links to [Assignment: organization-defined level of reduction].
    - ITEM-0884 SC-40(3) Imitative or Manipulative Communications Deception
      Related controls: [ITEM-0818], [ITEM-0823], [ITEM-0918].
      - REQ-1379 Implement cryptographic mechanisms to identify and reject wireless transmissions that are deliberate attempts to achieve imitative or manipulative communications deception based on signal parameters.
    - ITEM-0885 SC-40(4) Signal Parameter Identification
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1380 Implement cryptographic mechanisms to prevent the identification of [Assignment: organization-defined wireless transmitters] by using the transmitter signal parameters.
  - ITEM-0886 SC-41 Port and I/O Device Access
    Related controls: [ITEM-0118], [ITEM-0483].
    - REQ-1381 [Selection: physically; logically] disable or remove [Assignment: organization-defined connection ports or input/output devices] on the following systems or system components: [Assignment: organization-defined systems or system components].
  - ITEM-0887 SC-42 Sensor Capability and Data
    Related controls: [ITEM-0824].
    - REQ-1382 (a) Prohibit [Selection (one or more): the use of devices possessing [Assignment: organization-defined environmental sensing capabilities] in [Assignment: organization-defined facilities, areas, or systems] ; the remote activation of environmental sensing capabilities on organizational systems or system components with the following exceptions: [Assignment: organization-defined exceptions where remote activation of sensors is allowed] ] ; and
    - REQ-1383 (b) Provide an explicit indication of sensor use to [Assignment: organization-defined group of users].
    - ITEM-0888 SC-42(1) Reporting to Authorized Individuals or Roles
      - REQ-1384 Verify that the system is configured so that data or information collected by the [Assignment: organization-defined sensors] is only reported to authorized individuals or roles.
    - ITEM-0889 SC-42(2) Authorized Use
      Related controls: [ITEM-0612].
      - REQ-1385 Employ the following measures so that data or information collected by [Assignment: organization-defined sensors] is only used for authorized purposes: [Assignment: organization-defined measures].
    - ITEM-0890 SC-42(4) Notice of Collection
      Related controls: [ITEM-0611], [ITEM-0618], [ITEM-0622].
      - REQ-1386 Employ the following measures to facilitate an individual’s awareness that personally identifiable information is being collected by [Assignment: organization-defined sensors]: [Assignment: organization-defined measures].
    - ITEM-0891 SC-42(5) Collection Minimization
      Related controls: [ITEM-0675], [ITEM-0972].
      - REQ-1387 Employ [Assignment: organization-defined sensors] that are configured to minimize the collection of information about individuals that is not needed.
  - ITEM-0892 SC-43 Usage Restrictions
    Related controls: [ITEM-0110], [ITEM-0115], [ITEM-0256], [ITEM-0782], [ITEM-0833].
    - REQ-1388 (a) Establish usage restrictions and implementation guidelines for the following system components: [Assignment: organization-defined components] ; and
    - REQ-1389 (b) Authorize, monitor, and control the use of such components within the system.
  - ITEM-0893 SC-44 Detonation Chambers
    Related controls: [ITEM-0782], [ITEM-0833], [ITEM-0848], [ITEM-0849], [ITEM-0857], [ITEM-0871], [ITEM-0878], [ITEM-0913], [ITEM-0947].
    - REQ-1390 Employ a detonation chamber capability within [Assignment: organization-defined system, system component, or location].
  - ITEM-0894 SC-45 System Time Synchronization
    Related controls: [ITEM-0016], [ITEM-0174], [ITEM-0341], [ITEM-0379].
    - REQ-1391 Synchronize system clocks within and between systems and system components.
    - ITEM-0895 SC-45(1) Synchronization with Authoritative Time Source
      - REQ-1392 (a) Compare the internal system clocks [Assignment: organization-defined frequency] with [Assignment: organization-defined authoritative time source] ; and
      - REQ-1393 (b) Synchronize the internal system clocks to the authoritative time source when the time difference is greater than [Assignment: organization-defined time period].
    - ITEM-0896 SC-45(2) Secondary Authoritative Time Source
      - REQ-1394 (a) Identify a secondary authoritative time source that is in a different geographic region than the primary authoritative time source; and
      - REQ-1395 (b) Synchronize the internal system clocks to the secondary authoritative time source if the primary authoritative time source is unavailable.
  - ITEM-0897 SC-46 Cross Domain Policy Enforcement
    Related controls: [ITEM-0030], [ITEM-0782].
    - REQ-1396 Implement a policy enforcement mechanism [Selection: physically; logically] between the physical and/or network interfaces for the connecting security domains.
  - ITEM-0898 SC-47 Alternate Communications Paths
    Related controls: [ITEM-0291], [ITEM-0318].
    - REQ-1397 Establish [Assignment: organization-defined alternate communication paths] for system operations organizational command and control.
  - ITEM-0899 SC-48 Sensor Relocation
    Related controls: [ITEM-0151], [ITEM-0782], [ITEM-0918].
    - REQ-1398 Relocate [Assignment: organization-defined sensors and monitoring capabilities] to [Assignment: organization-defined locations] under the following conditions or circumstances: [Assignment: organization-defined conditions or circumstances].
    - ITEM-0900 SC-48(1) Dynamic Relocation of Sensors or Monitoring Capabilities
      - REQ-1399 Dynamically relocate [Assignment: organization-defined sensors and monitoring capabilities] to [Assignment: organization-defined locations] under the following conditions or circumstances: [Assignment: organization-defined conditions or circumstances].
  - ITEM-0901 SC-49 Hardware-enforced Separation and Policy Enforcement
    Related controls: [ITEM-0030], [ITEM-0675], [ITEM-0902].
    - REQ-1400 Implement hardware-enforced separation and policy enforcement mechanisms between [Assignment: organization-defined security domains].
  - ITEM-0902 SC-50 Software-enforced Separation and Policy Enforcement
    Related controls: [ITEM-0016], [ITEM-0030], [ITEM-0675], [ITEM-0766], [ITEM-0769], [ITEM-0901].
    - REQ-1401 Implement software-enforced separation and policy enforcement mechanisms between [Assignment: organization-defined security domains].
  - ITEM-0903 SC-51 Hardware-based Protection
    - REQ-1402 (a) Employ hardware-based, write-protect for [Assignment: organization-defined system firmware components] ; and
    - REQ-1403 (b) Implement specific procedures for [Assignment: organization-defined authorized individuals] to manually disable hardware write-protect for firmware modifications and re-enable the write-protect prior to returning to operational mode.
- ITEM-0904 System and Information Integrity
  The system and information integrity family (SI) of NIST SP 800-53 Revision 5.
  - ITEM-0905 SI-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0608], [ITEM-0675], [ITEM-0972].
    - REQ-1404 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high #privacy
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] system and information integrity policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the system and information integrity policy and the associated system and information integrity controls;
    - REQ-1405 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the system and information integrity policy and procedures; and #low #moderate #high #privacy
    - REQ-1406 (c) Review and update the current system and information integrity: #low #moderate #high #privacy
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-0906 SI-2 Flaw Remediation
    Related controls: [ITEM-0215], [ITEM-0239], [ITEM-0248], [ITEM-0251], [ITEM-0256], [ITEM-0269], [ITEM-0442], [ITEM-0641], [ITEM-0675], [ITEM-0718], [ITEM-0726], [ITEM-0913], [ITEM-0942], [ITEM-0947], [ITEM-0971].
    - REQ-1407 (a) Identify, report, and correct system flaws; #low #moderate #high
    - REQ-1408 (b) Test software and firmware updates related to flaw remediation for effectiveness and potential side effects before installation; #low #moderate #high
    - REQ-1409 (c) Install security-relevant software and firmware updates within [Assignment: organization-defined time period] of the release of the updates; and #low #moderate #high
    - REQ-1410 (d) Incorporate flaw remediation into the organizational configuration management process. #low #moderate #high
    - ITEM-0907 SI-2(2) Automated Flaw Remediation Status
      Related controls: [ITEM-0220], [ITEM-0918].
      - REQ-1411 Determine if system components have applicable security-relevant software and firmware updates installed using [Assignment: organization-defined automated mechanisms] [Assignment: organization-defined frequency]. #moderate #high
    - ITEM-0908 SI-2(3) Time to Remediate Flaws and Benchmarks for Corrective Actions
      - REQ-1412 (a) Measure the time between flaw identification and flaw remediation; and
      - REQ-1413 (b) Establish the following benchmarks for taking corrective actions: [Assignment: organization-defined benchmarks].
    - ITEM-0909 SI-2(4) Automated Patch Management Tools
      - REQ-1414 Employ automated patch management tools to facilitate flaw remediation to the following system components: [Assignment: organization-defined components].
    - ITEM-0910 SI-2(5) Automatic Software and Firmware Updates
      - REQ-1415 Install [Assignment: organization-defined security-relevant software and firmware updates] automatically to [Assignment: organization-defined system components].
    - ITEM-0911 SI-2(6) Removal of Previous Versions of Software and Firmware
      - REQ-1416 Remove previous versions of [Assignment: organization-defined software and firmware components] after updated versions have been installed.
    - ITEM-0912 SI-2(7) Root Cause Analysis
      Related controls: [ITEM-0002], [ITEM-0134], [ITEM-0150], [ITEM-0207], [ITEM-0233], [ITEM-0290], [ITEM-0340], [ITEM-0400], [ITEM-0441], [ITEM-0470], [ITEM-0491], [ITEM-0543], [ITEM-0555], [ITEM-0593], [ITEM-0611], [ITEM-0633], [ITEM-0656], [ITEM-0765], [ITEM-0905], [ITEM-1008], [ITEM-0151], [ITEM-0220], [ITEM-0427], [ITEM-0434], [ITEM-0444], [ITEM-0641], [ITEM-0726], [ITEM-0736], [ITEM-1026].
      - REQ-1417 (a) Conduct root cause analysis to identify underlying causes of issues or failures.
      - REQ-1418 (b) Develop actions to address the root cause of the issue or failure.
      - REQ-1419 (c) Implement the actions and monitor the implementation for effectiveness.
  - ITEM-0913 SI-3 Malicious Code Protection
    Related controls: [ITEM-0030], [ITEM-0115], [ITEM-0239], [ITEM-0269], [ITEM-0409], [ITEM-0444], [ITEM-0451], [ITEM-0551], [ITEM-0641], [ITEM-0782], [ITEM-0843], [ITEM-0849], [ITEM-0851], [ITEM-0893], [ITEM-0906], [ITEM-0918], [ITEM-0947], [ITEM-0961], [ITEM-0985].
    - REQ-1420 (a) Implement [Selection (one or more): signature-based; non-signature-based] malicious code protection mechanisms at system entry and exit points to detect and eradicate malicious code; #low #moderate #high
    - REQ-1421 (b) Automatically update malicious code protection mechanisms as new releases are available in accordance with organizational configuration management policy and procedures; #low #moderate #high
    - REQ-1422 (c) Configure malicious code protection mechanisms to: #low #moderate #high
      (c)(1) Perform periodic scans of the system [Assignment: organization-defined frequency] and real-time scans of files from external sources at [Selection (one or more): endpoint; network entry and exit points] as the files are downloaded, opened, or executed in accordance with organizational policy; and
      (c)(2) [Selection (one or more): block malicious code; quarantine malicious code; take [Assignment: organization-defined action] ] ; and send alert to [Assignment: organization-defined personnel or roles] in response to malicious code detection; and
    - REQ-1423 (d) Address the receipt of false positives during malicious code detection and eradication and the resulting potential impact on the availability of the system. #low #moderate #high
    - ITEM-0914 SI-3(4) Updates Only by Privileged Users
      Related controls: [ITEM-0251].
      - REQ-1424 Update malicious code protection mechanisms only when directed by a privileged user.
    - ITEM-0915 SI-3(6) Testing and Verification
      Related controls: [ITEM-0208], [ITEM-0220], [ITEM-0641].
      - REQ-1425 (a) Test malicious code protection mechanisms [Assignment: organization-defined frequency] by introducing known benign code into the system; and
      - REQ-1426 (b) Verify that the detection of the code and the associated incident reporting occur.
    - ITEM-0916 SI-3(8) Detect Unauthorized Commands
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0190].
      - REQ-1427 (a) Detect the following unauthorized operating system commands through the kernel application programming interface on [Assignment: organization-defined system hardware components]: [Assignment: organization-defined unauthorized operating system commands] ; and
      - REQ-1428 (b) [Selection (one or more): issue a warning; audit the command execution; prevent the execution of the command].
    - ITEM-0917 SI-3(10) Malicious Code Analysis
      - REQ-1429 (a) Employ the following tools and techniques to analyze the characteristics and behavior of malicious code: [Assignment: organization-defined tools and techniques] ; and
      - REQ-1430 (b) Incorporate the results from malicious code analysis into organizational incident response and flaw remediation processes.
  - ITEM-0918 SI-4 System Monitoring
    Related controls: [ITEM-0003], [ITEM-0016], [ITEM-0030], [ITEM-0077], [ITEM-0102], [ITEM-0151], [ITEM-0163], [ITEM-0172], [ITEM-0175], [ITEM-0190], [ITEM-0195], [ITEM-0199], [ITEM-0220], [ITEM-0239], [ITEM-0256], [ITEM-0269], [ITEM-0282], [ITEM-0386], [ITEM-0409], [ITEM-0444], [ITEM-0451], [ITEM-0551], [ITEM-0568], [ITEM-0641], [ITEM-0654], [ITEM-0777], [ITEM-0782], [ITEM-0833], [ITEM-0849], [ITEM-0862], [ITEM-0871], [ITEM-0872], [ITEM-0875], [ITEM-0892], [ITEM-0913], [ITEM-0944], [ITEM-0947], [ITEM-1027], [ITEM-1029].
    - REQ-1431 (a) Monitor the system to detect: #low #moderate #high
      (a)(1) Attacks and indicators of potential attacks in accordance with the following monitoring objectives: [Assignment: organization-defined monitoring objectives] ; and
      (a)(2) Unauthorized local, network, and remote connections;
    - REQ-1432 (b) Identify unauthorized use of the system through the following techniques and methods: [Assignment: organization-defined techniques and methods]; #low #moderate #high
    - REQ-1433 (c) Invoke internal monitoring capabilities or deploy monitoring devices: #low #moderate #high
      (c)(1) Strategically within the system to collect organization-determined essential information; and
      (c)(2) At ad hoc locations within the system to track specific types of transactions of interest to the organization;
    - REQ-1434 (d) Analyze detected events and anomalies; #low #moderate #high
    - REQ-1435 (e) Adjust the level of system monitoring activity when there is a change in risk to organizational operations and assets, individuals, other organizations, or the Nation; #low #moderate #high
    - REQ-1436 (f) Obtain legal opinion regarding system monitoring activities; and #low #moderate #high
    - REQ-1437 (g) Provide [Assignment: organization-defined system monitoring information] to [Assignment: organization-defined personnel or roles] [Selection (one or more): as needed; [Assignment: organization-defined frequency] ]. #low #moderate #high
    - ITEM-0919 SI-4(1) System-wide Intrusion Detection System
      - REQ-1438 Connect and configure individual intrusion detection tools into a system-wide intrusion detection system.
    - ITEM-0920 SI-4(2) Automated Tools and Mechanisms for Real-time Analysis
      Related controls: [ITEM-0581], [ITEM-0583].
      - REQ-1439 Employ automated tools and mechanisms to support near real-time analysis of events. #moderate #high
    - ITEM-0921 SI-4(3) Automated Tool and Mechanism Integration
      Related controls: [ITEM-0581], [ITEM-0583].
      - REQ-1440 Employ automated tools and mechanisms to integrate intrusion detection tools and mechanisms into access control and flow control mechanisms.
    - ITEM-0922 SI-4(4) Inbound and Outbound Communications Traffic
      - REQ-1441 (a) Determine criteria for unusual or unauthorized activities or conditions for inbound and outbound communications traffic; #moderate #high
      - REQ-1442 (b) Monitor inbound and outbound communications traffic [Assignment: organization-defined frequency] for [Assignment: organization-defined unusual or unauthorized activities or conditions]. #moderate #high
    - ITEM-0923 SI-4(5) System-generated Alerts
      Related controls: [ITEM-0155], [ITEM-0157], [ITEM-0507].
      - REQ-1443 Alert [Assignment: organization-defined personnel or roles] when the following system-generated indications of compromise or potential compromise occur: [Assignment: organization-defined compromise indicators]. #moderate #high
    - ITEM-0924 SI-4(7) Automated Response to Suspicious Events
      - REQ-1444 (a) Notify [Assignment: organization-defined incident response personnel] of detected suspicious events; and
      - REQ-1445 (b) Take the following actions upon detection: [Assignment: organization-defined least-disruptive actions].
    - ITEM-0925 SI-4(9) Testing of Monitoring Tools and Mechanisms
      - REQ-1446 Test intrusion-monitoring tools and mechanisms [Assignment: organization-defined frequency].
    - ITEM-0926 SI-4(10) Visibility of Encrypted Communications
      - REQ-1447 Make provisions so that [Assignment: organization-defined encrypted communications traffic] is visible to [Assignment: organization-defined system monitoring tools and mechanisms]. #high
    - ITEM-0927 SI-4(11) Analyze Communications Traffic Anomalies
      - REQ-1448 Analyze outbound communications traffic at the external interfaces to the system and selected [Assignment: organization-defined interior points] to discover anomalies.
    - ITEM-0928 SI-4(12) Automated Organization-generated Alerts
      - REQ-1449 Alert [Assignment: organization-defined personnel or roles] using [Assignment: organization-defined automated mechanisms] when the following indications of inappropriate or unusual activities with security or privacy implications occur: [Assignment: organization-defined activities that trigger alerts]. #high
    - ITEM-0929 SI-4(13) Analyze Traffic and Event Patterns
      - REQ-1450 (a) Analyze communications traffic and event patterns for the system;
      - REQ-1451 (b) Develop profiles representing common traffic and event patterns; and
      - REQ-1452 (c) Use the traffic and event profiles in tuning system-monitoring devices.
    - ITEM-0930 SI-4(14) Wireless Intrusion Detection
      Related controls: [ITEM-0110], [ITEM-0350].
      - REQ-1453 Employ a wireless intrusion detection system to identify rogue wireless devices and to detect attack attempts and potential compromises or breaches to the system. #high
    - ITEM-0931 SI-4(15) Wireless to Wireline Communications
      Related controls: [ITEM-0110].
      - REQ-1454 Employ an intrusion detection system to monitor wireless communications traffic as the traffic passes from wireless to wireline networks.
    - ITEM-0932 SI-4(16) Correlate Monitoring Information
      Related controls: [ITEM-0163].
      - REQ-1455 Correlate information from monitoring tools and mechanisms employed throughout the system.
    - ITEM-0933 SI-4(17) Integrated Situational Awareness
      Related controls: [ITEM-0202], [ITEM-0507], [ITEM-1009], [ITEM-1015], [ITEM-1023].
      - REQ-1456 Correlate information from monitoring physical, cyber, and supply chain activities to achieve integrated, organization-wide situational awareness.
    - ITEM-0934 SI-4(18) Analyze Traffic and Covert Exfiltration
      - REQ-1457 Analyze outbound communications traffic at external interfaces to the system and at the following interior points to detect covert exfiltration of information: [Assignment: organization-defined interior points].
    - ITEM-0935 SI-4(19) Risk for Individuals
      - REQ-1458 Implement [Assignment: organization-defined additional monitoring] of individuals who have been identified by [Assignment: organization-defined sources] as posing an increased level of risk.
    - ITEM-0936 SI-4(20) Privileged Users
      Related controls: [ITEM-0110].
      - REQ-1459 Implement the following additional monitoring of privileged users: [Assignment: organization-defined additional monitoring]. #high
    - ITEM-0937 SI-4(21) Probationary Periods
      Related controls: [ITEM-0110].
      - REQ-1460 Implement the following additional monitoring of individuals during [Assignment: organization-defined probationary period]: [Assignment: organization-defined additional monitoring].
    - ITEM-0938 SI-4(22) Unauthorized Network Services
      Related controls: [ITEM-0259].
      - REQ-1461 (a) Detect network services that have not been authorized or approved by [Assignment: organization-defined authorization or approval processes] ; and #high
      - REQ-1462 (b) [Selection (one or more): audit; alert [Assignment: organization-defined personnel or roles] ] when detected. #high
    - ITEM-0939 SI-4(23) Host-based Devices
      Related controls: [ITEM-0110], [ITEM-0115].
      - REQ-1463 Implement the following host-based monitoring mechanisms at [Assignment: organization-defined system components]: [Assignment: organization-defined host-based monitoring mechanisms].
    - ITEM-0940 SI-4(24) Indicators of Compromise
      Related controls: [ITEM-0110].
      - REQ-1464 Discover, collect, and distribute to [Assignment: organization-defined personnel or roles] , indicators of compromise provided by [Assignment: organization-defined sources].
    - ITEM-0941 SI-4(25) Optimize Network Traffic Analysis
      - REQ-1465 Provide visibility into network traffic at external and key internal system interfaces to optimize the effectiveness of monitoring devices.
  - ITEM-0942 SI-5 Security Alerts, Advisories, and Directives
    Related controls: [ITEM-0571], [ITEM-0641], [ITEM-0906].
    - REQ-1466 (a) Receive system security alerts, advisories, and directives from [Assignment: organization-defined external organizations] on an ongoing basis; #low #moderate #high
    - REQ-1467 (b) Generate internal security alerts, advisories, and directives as deemed necessary; #low #moderate #high
    - REQ-1468 (c) Disseminate security alerts, advisories, and directives to: [Selection (one or more): [Assignment: organization-defined personnel or roles] ; [Assignment: organization-defined elements] ; [Assignment: organization-defined external organizations] ] ; and #low #moderate #high
    - REQ-1469 (d) Implement security directives in accordance with established time frames, or notify the issuing organization of the degree of noncompliance. #low #moderate #high
    - ITEM-0943 SI-5(1) Automated Alerts and Advisories
      - REQ-1470 Broadcast security alert and advisory information throughout the organization using [Assignment: organization-defined automated mechanisms]. #high
  - ITEM-0944 SI-6 Security and Privacy Function Verification
    Related controls: [ITEM-0220], [ITEM-0248], [ITEM-0256], [ITEM-0947].
    - REQ-1471 (a) Verify the correct operation of [Assignment: organization-defined security and privacy functions]; #high
    - REQ-1472 (b) Perform the verification of the functions specified in SI-6a [Selection (one or more): [Assignment: organization-defined system transitional states] ; upon command by user with appropriate privilege; [Assignment: organization-defined frequency] ]; #high
    - REQ-1473 (c) Alert [Assignment: organization-defined personnel or roles] to failed security and privacy verification tests; and #high
    - REQ-1474 (d) [Selection (one or more): shut the system down; restart the system; [Assignment: organization-defined alternative action(s)] ] when anomalies are discovered. #high
    - ITEM-0945 SI-6(2) Automation Support for Distributed Testing
      Related controls: [ITEM-0906].
      - REQ-1475 Implement automated mechanisms to support the management of distributed security and privacy function testing.
    - ITEM-0946 SI-6(3) Report Verification Results
      Related controls: [ITEM-0918], [ITEM-1015], [ITEM-1020].
      - REQ-1476 Report the results of security and privacy function verification to [Assignment: organization-defined personnel or roles].
  - ITEM-0947 SI-7 Software, Firmware, and Information Integrity
    Related controls: [ITEM-0030], [ITEM-0239], [ITEM-0259], [ITEM-0269], [ITEM-0444], [ITEM-0451], [ITEM-0641], [ITEM-0675], [ITEM-0709], [ITEM-0718], [ITEM-0809], [ITEM-0818], [ITEM-0823], [ITEM-0851], [ITEM-0875], [ITEM-0913], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1023], [ITEM-1027], [ITEM-1029], [ITEM-1030].
    - REQ-1477 (a) Employ integrity verification tools to detect unauthorized changes to the following software, firmware, and information: [Assignment: organization-defined software, firmware, and information] ; and #moderate #high
    - REQ-1478 (b) Take the following actions when unauthorized changes to the software, firmware, and information are detected: [Assignment: organization-defined actions]. #moderate #high
    - ITEM-0948 SI-7(1) Integrity Checks
      - REQ-1479 Perform an integrity check of [Assignment: organization-defined software, firmware, and information] [Selection (one or more): at startup; at [Assignment: organization-defined transitional states or security-relevant events] ; [Assignment: organization-defined frequency] ]. #moderate #high
    - ITEM-0949 SI-7(2) Automated Notifications of Integrity Violations
      - REQ-1480 Employ automated tools that provide notification to [Assignment: organization-defined personnel or roles] upon discovering discrepancies during integrity verification. #high
    - ITEM-0950 SI-7(3) Centrally Managed Integrity Tools
      Related controls: [ITEM-0152], [ITEM-0906], [ITEM-0961].
      - REQ-1481 Employ centrally managed integrity verification tools.
    - ITEM-0951 SI-7(5) Automated Response to Integrity Violations
      - REQ-1482 Automatically [Selection (one or more): shut down the system; restart the system; implement [Assignment: organization-defined controls] ] when integrity violations are discovered. #high
    - ITEM-0952 SI-7(6) Cryptographic Protection
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1483 Implement cryptographic mechanisms to detect unauthorized changes to software, firmware, and information.
    - ITEM-0953 SI-7(7) Integration of Detection and Response
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0409], [ITEM-0425], [ITEM-0918].
      - REQ-1484 Incorporate the detection of the following unauthorized changes into the organizational incident response capability: [Assignment: organization-defined changes]. #moderate #high
    - ITEM-0954 SI-7(8) Auditing Capability for Significant Events
      Related controls: [ITEM-0151], [ITEM-0163], [ITEM-0190].
      - REQ-1485 Upon detection of a potential integrity violation, provide the capability to audit the event and initiate the following actions: [Selection (one or more): generate an audit record; alert current user; alert [Assignment: organization-defined personnel or roles] ; [Assignment: organization-defined other actions] ].
    - ITEM-0955 SI-7(9) Verify Boot Process
      Related controls: [ITEM-0944].
      - REQ-1486 Verify the integrity of the boot process of the following system components: [Assignment: organization-defined system components].
    - ITEM-0956 SI-7(10) Protection of Boot Firmware
      Related controls: [ITEM-0944].
      - REQ-1487 Implement the following mechanisms to protect the integrity of boot firmware in [Assignment: organization-defined system components]: [Assignment: organization-defined mechanisms].
    - ITEM-0957 SI-7(12) Integrity Verification
      Related controls: [ITEM-0906], [ITEM-0282].
      - REQ-1488 Require that the integrity of the following user-installed software be verified prior to execution: [Assignment: organization-defined user-installed software].
    - ITEM-0958 SI-7(15) Code Authentication
      Related controls: [ITEM-0251], [ITEM-0818], [ITEM-0823].
      - REQ-1489 Implement cryptographic mechanisms to authenticate the following software or firmware components prior to installation: [Assignment: organization-defined software or firmware components]. #high
    - ITEM-0959 SI-7(16) Time Limit on Process Execution Without Supervision
      - REQ-1490 Prohibit processes from executing without supervision for more than [Assignment: organization-defined time period].
    - ITEM-0960 SI-7(17) Runtime Application Self-protection
      Related controls: [ITEM-0986].
      - REQ-1491 Implement [Assignment: organization-defined controls] for application self-protection at runtime.
  - ITEM-0961 SI-8 Spam Protection
    Related controls: [ITEM-0551], [ITEM-0777], [ITEM-0782], [ITEM-0877], [ITEM-0913], [ITEM-0918].
    - REQ-1492 (a) Employ spam protection mechanisms at system entry and exit points to detect and act on unsolicited messages; and #moderate #high
    - REQ-1493 (b) Update spam protection mechanisms when new releases are available in accordance with organizational configuration management policy and procedures. #moderate #high
    - ITEM-0962 SI-8(2) Automatic Updates
      - REQ-1494 Automatically update spam protection mechanisms [Assignment: organization-defined frequency]. #moderate #high
    - ITEM-0963 SI-8(3) Continuous Learning Capability
      - REQ-1495 Implement spam protection mechanisms with a learning capability to more effectively identify legitimate communications traffic.
  - ITEM-0964 SI-10 Information Input Validation
    - REQ-1496 Check the validity of the following information inputs: [Assignment: organization-defined information inputs]. #moderate #high
    - ITEM-0965 SI-10(1) Manual Override Capability
      Related controls: [ITEM-0016], [ITEM-0151], [ITEM-0190].
      - REQ-1497 (a) Provide a manual override capability for input validation of the following information inputs: [Assignment: organization-defined information inputs];
      - REQ-1498 (b) Restrict the use of the manual override capability to only [Assignment: organization-defined authorized individuals] ; and
      - REQ-1499 (c) Audit the use of the manual override capability.
    - ITEM-0966 SI-10(2) Review and Resolve Errors
      - REQ-1500 Review and resolve input validation errors within [Assignment: organization-defined time period].
    - ITEM-0967 SI-10(3) Predictable Behavior
      - REQ-1501 Verify that the system behaves in a predictable and documented manner when invalid inputs are received.
    - ITEM-0968 SI-10(4) Timing Interactions
      - REQ-1502 Account for timing interactions among system components in determining appropriate responses for invalid inputs.
    - ITEM-0969 SI-10(5) Restrict Inputs to Trusted Sources and Approved Formats
      Related controls: [ITEM-0016], [ITEM-0062].
      - REQ-1503 Restrict the use of information inputs to [Assignment: organization-defined trusted sources] and/or [Assignment: organization-defined formats].
    - ITEM-0970 SI-10(6) Injection Prevention
      Related controls: [ITEM-0016], [ITEM-0062].
      - REQ-1504 Prevent untrusted data injections.
  - ITEM-0971 SI-11 Error Handling
    Related controls: [ITEM-0151], [ITEM-0152], [ITEM-0862], [ITEM-0906], [ITEM-0985].
    - REQ-1505 (a) Generate error messages that provide information necessary for corrective actions without revealing information that could be exploited; and #moderate #high
    - REQ-1506 (b) Reveal error messages only to [Assignment: organization-defined personnel or roles]. #moderate #high
  - ITEM-0972 SI-12 Information Management and Retention
    Related controls: [ITEM-0091], [ITEM-0157], [ITEM-0188], [ITEM-0208], [ITEM-0212], [ITEM-0215], [ITEM-0217], [ITEM-0220], [ITEM-0230], [ITEM-0251], [ITEM-0278], [ITEM-0291], [ITEM-0434], [ITEM-0471], [ITEM-0472], [ITEM-0473], [ITEM-0477], [ITEM-0544], [ITEM-0545], [ITEM-0558], [ITEM-0564], [ITEM-0565], [ITEM-0594], [ITEM-0604], [ITEM-0612], [ITEM-0615], [ITEM-0634], [ITEM-0636], [ITEM-0674], [ITEM-0675], [ITEM-1009].
    - REQ-1507 Manage and retain information within the system and information output from the system in accordance with applicable laws, executive orders, directives, regulations, policies, standards, guidelines and operational requirements. #low #moderate #high #privacy
    - ITEM-0973 SI-12(1) Limit Personally Identifiable Information Elements
      Related controls: [ITEM-0583].
      - REQ-1508 Limit personally identifiable information being processed in the information life cycle to the following elements of personally identifiable information: [Assignment: organization-defined elements of personally identifiable information]. #privacy
    - ITEM-0974 SI-12(2) Minimize Personally Identifiable Information in Testing, Training, and Research
      Related controls: [ITEM-0580], [ITEM-0583], [ITEM-0994].
      - REQ-1509 Use the following techniques to minimize the use of personally identifiable information for research, testing, or training: [Assignment: organization-defined techniques]. #privacy
    - ITEM-0975 SI-12(3) Information Disposal
      - REQ-1510 Use the following techniques to dispose of, destroy, or erase information following the retention period: [Assignment: organization-defined techniques]. #privacy
  - ITEM-0976 SI-13 Predictable Failure Prevention
    Related controls: [ITEM-0291], [ITEM-0332], [ITEM-0338], [ITEM-0442], [ITEM-0464], [ITEM-0675], [ITEM-0781].
    - REQ-1511 (a) Determine mean time to failure (MTTF) for the following system components in specific environments of operation: [Assignment: organization-defined system components] ; and
    - REQ-1512 (b) Provide substitute system components and a means to exchange active and standby components in accordance with the following criteria: [Assignment: organization-defined mean time to failure (MTTF) substitution criteria].
    - ITEM-0977 SI-13(1) Transferring Component Responsibilities
      - REQ-1513 Take system components out of service by transferring component responsibilities to substitute components no later than [Assignment: organization-defined fraction or percentage] of mean time to failure.
    - ITEM-0978 SI-13(3) Manual Transfer Between Components
      - REQ-1514 Manually initiate transfers between active and standby system components when the use of the active component reaches [Assignment: organization-defined percentage] of the mean time to failure.
    - ITEM-0979 SI-13(4) Standby Component Installation and Notification
      - REQ-1515 (a) Ensure that the standby components are successfully and transparently installed within [Assignment: organization-defined time period] ; and
      - REQ-1516 (b) [Selection (one or more): activate [Assignment: organization-defined alarm] ; automatically shut down the system; [Assignment: organization-defined action] ].
    - ITEM-0980 SI-13(5) Failover Capability
      Related controls: [ITEM-0308], [ITEM-0312], [ITEM-0324].
      - REQ-1517 Provide [Selection: real-time; near real-time] [Assignment: organization-defined failover capability] for the system.
  - ITEM-0981 SI-14 Non-persistence
    Related controls: [ITEM-0857], [ITEM-0868], [ITEM-1004].
    - REQ-1518 Implement non-persistent [Assignment: organization-defined system components and services] that are initiated in a known state and terminated [Selection (one or more): upon end of session of use; [Assignment: organization-defined frequency] ].
    - ITEM-0982 SI-14(1) Refresh from Trusted Sources
      - REQ-1519 Obtain software and data employed during system component and service refreshes from the following trusted sources: [Assignment: organization-defined trusted sources].
    - ITEM-0983 SI-14(2) Non-persistent Information
      - REQ-1520 (a) [Selection: refresh [Assignment: organization-defined information] [Assignment: organization-defined frequency] ; generate [Assignment: organization-defined information] on demand] ; and
      - REQ-1521 (b) Delete information when no longer needed.
    - ITEM-0984 SI-14(3) Non-persistent Connectivity
      Related controls: [ITEM-0815].
      - REQ-1522 Establish connections to the system on demand and terminate connections after [Selection: completion of a request; a period of non-use].
  - ITEM-0985 SI-15 Information Output Filtering
    Related controls: [ITEM-0913], [ITEM-0918], [ITEM-0971].
    - REQ-1523 Validate information output from the following software programs and/or applications to ensure that the information is consistent with the expected content: [Assignment: organization-defined software programs and/or applications].
  - ITEM-0986 SI-16 Memory Protection
    Related controls: [ITEM-0132], [ITEM-0769], [ITEM-0947].
    - REQ-1524 Implement the following controls to protect the system memory from unauthorized code execution: [Assignment: organization-defined controls]. #moderate #high
  - ITEM-0987 SI-17 Fail-safe Procedures
    Related controls: [ITEM-0337], [ITEM-0338], [ITEM-0847], [ITEM-0976].
    - REQ-1525 Implement the indicated fail-safe procedures when the indicated failures occur: [Assignment: organization-defined list of failure conditions and associated fail-safe procedures].
  - ITEM-0988 SI-18 Personally Identifiable Information Quality Operations
    Related controls: [ITEM-0580], [ITEM-0582], [ITEM-0612], [ITEM-0918].
    - REQ-1526 (a) Check the accuracy, relevance, timeliness, and completeness of personally identifiable information across the information life cycle [Assignment: organization-defined frequency] ; and #privacy
    - REQ-1527 (b) Correct or delete inaccurate or outdated personally identifiable information. #privacy
    - ITEM-0989 SI-18(1) Automation Support
      Related controls: [ITEM-0575], [ITEM-0652].
      - REQ-1528 Correct or delete personally identifiable information that is inaccurate or outdated, incorrectly determined regarding impact, or incorrectly de-identified using [Assignment: organization-defined automated mechanisms].
    - ITEM-0990 SI-18(2) Data Tags
      Related controls: [ITEM-0016], [ITEM-0091], [ITEM-0828].
      - REQ-1529 Employ data tags to automate the correction or deletion of personally identifiable information across the information life cycle within organizational systems.
    - ITEM-0991 SI-18(3) Collection
      - REQ-1530 Collect personally identifiable information directly from the individual.
    - ITEM-0992 SI-18(4) Individual Requests
      - REQ-1531 Correct or delete personally identifiable information upon request by individuals or their designated representatives. #privacy
    - ITEM-0993 SI-18(5) Notice of Correction or Deletion
      - REQ-1532 Notify [Assignment: organization-defined recipients] and individuals that the personally identifiable information has been corrected or deleted.
  - ITEM-0994 SI-19 De-identification
    Related controls: [ITEM-0477], [ITEM-0580], [ITEM-0581], [ITEM-0582], [ITEM-0634], [ITEM-0972].
    - REQ-1533 (a) Remove the following elements of personally identifiable information from datasets: [Assignment: organization-defined elements] ; and #privacy
    - REQ-1534 (b) Evaluate [Assignment: organization-defined frequency] for effectiveness of de-identification. #privacy
    - ITEM-0995 SI-19(1) Collection
      - REQ-1535 De-identify the dataset upon collection by not collecting personally identifiable information.
    - ITEM-0996 SI-19(2) Archiving
      - REQ-1536 Prohibit archiving of personally identifiable information elements if those elements in a dataset will not be needed after the dataset is archived.
    - ITEM-0997 SI-19(3) Release
      - REQ-1537 Remove personally identifiable information elements from a dataset prior to its release if those elements in the dataset do not need to be part of the data release.
    - ITEM-0998 SI-19(4) Removal, Masking, Encryption, Hashing, or Replacement of Direct Identifiers
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1538 Remove, mask, encrypt, hash, or replace direct identifiers in a dataset.
    - ITEM-0999 SI-19(5) Statistical Disclosure Control
      - REQ-1539 Manipulate numerical data, contingency tables, and statistical findings so that no individual or organization is identifiable in the results of the analysis.
    - ITEM-1000 SI-19(6) Differential Privacy
      Related controls: [ITEM-0818], [ITEM-0823].
      - REQ-1540 Prevent disclosure of personally identifiable information by adding non-deterministic noise to the results of mathematical operations before the results are reported.
    - ITEM-1001 SI-19(7) Validated Algorithms and Software
      - REQ-1541 Perform de-identification using validated algorithms and software that is validated to implement the algorithms.
    - ITEM-1002 SI-19(8) Motivated Intruder
      - REQ-1542 Perform a motivated intruder test on the de-identified dataset to determine if the identified data remains or if the de-identified data can be re-identified.
  - ITEM-1003 SI-20 Tainting
    Related controls: [ITEM-0195].
    - REQ-1543 Embed data or capabilities in the following systems or system components to determine if organizational data has been exfiltrated or improperly removed from the organization: [Assignment: organization-defined systems or system components].
  - ITEM-1004 SI-21 Information Refresh
    Related controls: [ITEM-0981].
    - REQ-1544 Refresh [Assignment: organization-defined information] at [Assignment: organization-defined frequencies] or generate the information on demand and delete the information when no longer needed.
  - ITEM-1005 SI-22 Information Diversity
    - REQ-1545 (a) Identify the following alternative sources of information for [Assignment: organization-defined essential functions and services]: [Assignment: organization-defined alternative information sources] ; and
    - REQ-1546 (b) Use an alternative information source for the execution of essential functions or services on [Assignment: organization-defined systems or system components] when the primary source of information is corrupted or unavailable.
  - ITEM-1006 SI-23 Information Fragmentation
    - REQ-1547 (a) Fragment the following information: [Assignment: organization-defined information] ; and
    - REQ-1548 (b) Distribute the fragmented information across the following systems or system components: [Assignment: organization-defined systems or system components].
- ITEM-1007 Supply Chain Risk Management
  The supply chain risk management family (SR) of NIST SP 800-53 Revision 5.
  - ITEM-1008 SR-1 Policy and Procedures
    Related controls: [ITEM-0565], [ITEM-0588], [ITEM-0608], [ITEM-0972].
    - REQ-1549 (a) Develop, document, and disseminate to [Assignment: organization-defined personnel or roles]: #low #moderate #high
      (a)(1) [Selection (one or more): organization-level; mission/business process-level; system-level] supply chain risk management policy that:
      (a)(1)(a) Addresses purpose, scope, roles, responsibilities, management commitment, coordination among organizational entities, and compliance; and
      (a)(1)(b) Is consistent with applicable laws, executive orders, directives, regulations, policies, standards, and guidelines; and
      (a)(2) Procedures to facilitate the implementation of the supply chain risk management policy and the associated supply chain risk management controls;
    - REQ-1550 (b) Designate an [Assignment: organization-defined official] to manage the development, documentation, and dissemination of the supply chain risk management policy and procedures; and #low #moderate #high
    - REQ-1551 (c) Review and update the current supply chain risk management: #low #moderate #high
      (c)(1) Policy [Assignment: organization-defined frequency] and following [Assignment: organization-defined events] ; and
      (c)(2) Procedures [Assignment: organization-defined frequency] and following [Assignment: organization-defined events].
  - ITEM-1009 SR-2 Supply Chain Risk Management Plan
    Related controls: [ITEM-0208], [ITEM-0302], [ITEM-0409], [ITEM-0442], [ITEM-0464], [ITEM-0533], [ITEM-0544], [ITEM-0565], [ITEM-0588], [ITEM-0636], [ITEM-0651], [ITEM-0675], [ITEM-0918].
    - REQ-1552 (a) Develop a plan for managing supply chain risks associated with the research and development, design, manufacturing, acquisition, delivery, integration, operations and maintenance, and disposal of the following systems, system components or system services: [Assignment: organization-defined systems, system components, or system services]; #low #moderate #high
    - REQ-1553 (b) Review and update the supply chain risk management plan [Assignment: organization-defined frequency] or as required, to address threat, organizational or environmental changes; and #low #moderate #high
    - REQ-1554 (c) Protect the supply chain risk management plan from unauthorized disclosure and modification. #low #moderate #high
    - ITEM-1010 SR-2(1) Establish SCRM Team
      - REQ-1555 Establish a supply chain risk management team consisting of [Assignment: organization-defined personnel, roles and responsibilities] to lead and support the following SCRM activities: [Assignment: organization-defined supply chain risk management activities]. #low #moderate #high
  - ITEM-1011 SR-3 Supply Chain Controls and Processes
    Related controls: [ITEM-0208], [ITEM-0442], [ITEM-0464], [ITEM-0496], [ITEM-0533], [ITEM-0548], [ITEM-0588], [ITEM-0657], [ITEM-0658], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0709], [ITEM-0718], [ITEM-0736], [ITEM-0782], [ITEM-0855], [ITEM-0857], [ITEM-0877], [ITEM-0947], [ITEM-1023], [ITEM-1027], [ITEM-1030].
    - REQ-1556 (a) Establish a process or processes to identify and address weaknesses or deficiencies in the supply chain elements and processes of [Assignment: organization-defined system or system component] in coordination with [Assignment: organization-defined supply chain personnel]; #low #moderate #high
    - REQ-1557 (b) Employ the following controls to protect against supply chain risks to the system, system component, or system service and to limit the harm or consequences from supply chain-related events: [Assignment: organization-defined supply chain controls] ; and #low #moderate #high
    - REQ-1558 (c) Document the selected and implemented supply chain processes and controls in [Selection (one or more): security and privacy plans; supply chain risk management plan; [Assignment: organization-defined document] ]. #low #moderate #high
    - ITEM-1012 SR-3(1) Diverse Supply Base
      - REQ-1559 Employ a diverse set of sources for the following system components and services: [Assignment: organization-defined system components and services].
    - ITEM-1013 SR-3(2) Limitation of Harm
      - REQ-1560 Employ the following controls to limit harm from potential adversaries identifying and targeting the organizational supply chain: [Assignment: organization-defined controls].
    - ITEM-1014 SR-3(3) Sub-tier Flow Down
      Related controls: [ITEM-1020], [ITEM-1026].
      - REQ-1561 Ensure that the controls included in the contracts of prime contractors are also included in the contracts of subcontractors.
  - ITEM-1015 SR-4 Provenance
    Related controls: [ITEM-0269], [ITEM-0442], [ITEM-0464], [ITEM-0653], [ITEM-0658], [ITEM-0675], [ITEM-0918].
    - REQ-1562 Document, monitor, and maintain valid provenance of the following systems, system components, and associated data: [Assignment: organization-defined systems, system components, and associated data].
    - ITEM-1016 SR-4(1) Identity
      Related controls: [ITEM-0341], [ITEM-0379], [ITEM-0533].
      - REQ-1563 Establish and maintain unique identification of the following supply chain elements, processes, and personnel associated with the identified system and critical system components: [Assignment: organization-defined supply chain elements, processes, and personnel].
    - ITEM-1017 SR-4(2) Track and Trace
      Related controls: [ITEM-0341], [ITEM-0379], [ITEM-0533], [ITEM-0544].
      - REQ-1564 Establish and maintain unique identification of the following systems and critical system components for tracking through the supply chain: [Assignment: organization-defined systems and critical system components].
    - ITEM-1018 SR-4(3) Validate as Genuine and Not Altered
      Related controls: [ITEM-0142], [ITEM-1027], [ITEM-1029], [ITEM-1030].
      - REQ-1565 Employ the following controls to validate that the system or system component received is genuine and has not been altered: [Assignment: organization-defined controls].
    - ITEM-1019 SR-4(4) Supply Chain Integrity — Pedigree
      Related controls: [ITEM-0636].
      - REQ-1566 Employ [Assignment: organization-defined controls] and conduct [Assignment: organization-defined analysis method] to ensure the integrity of the system and system components by validating the internal composition and provenance of critical or mission-essential technologies, products, and services.
  - ITEM-1020 SR-5 Acquisition Strategies, Tools, and Methods
    Related controls: [ITEM-0142], [ITEM-0657], [ITEM-0658], [ITEM-0662], [ITEM-0674], [ITEM-0675], [ITEM-0709], [ITEM-0718], [ITEM-0736], [ITEM-1023], [ITEM-1027], [ITEM-1029], [ITEM-1030].
    - REQ-1567 Employ the following acquisition strategies, contract tools, and procurement methods to protect against, identify, and mitigate supply chain risks: [Assignment: organization-defined strategies, tools, and methods]. #low #moderate #high
    - ITEM-1021 SR-5(1) Adequate Supply
      Related controls: [ITEM-0653].
      - REQ-1568 Employ the following controls to ensure an adequate supply of [Assignment: organization-defined critical system components]: [Assignment: organization-defined controls].
    - ITEM-1022 SR-5(2) Assessments Prior to Selection, Acceptance, Modification, or Update
      Related controls: [ITEM-0226], [ITEM-0641], [ITEM-0726], [ITEM-0947].
      - REQ-1569 Assess the system, system component, or system service prior to selection, acceptance, modification, or update.
  - ITEM-1023 SR-6 Supplier Assessments and Reviews
    Related controls: [ITEM-1011], [ITEM-1020].
    - REQ-1570 Assess and review the supply chain-related risks associated with suppliers or contractors and the system, system component, or system service they provide [Assignment: organization-defined frequency]. #moderate #high
    - ITEM-1024 SR-6(1) Testing and Analysis
      Related controls: [ITEM-0226], [ITEM-0918].
      - REQ-1571 Employ [Selection (one or more): organizational analysis; independent third-party analysis; organizational testing; independent third-party testing] of the following supply chain elements, processes, and actors associated with the system, system component, or system service: [Assignment: organization-defined supply chain elements, processes, and actors].
  - ITEM-1025 SR-7 Supply Chain Operations Security
    Related controls: [ITEM-0877].
    - REQ-1572 Employ the following Operations Security (OPSEC) controls to protect supply chain-related information for the system, system component, or system service: [Assignment: organization-defined OPSEC controls].
  - ITEM-1026 SR-8 Notification Agreements
    Related controls: [ITEM-0409], [ITEM-0427], [ITEM-0434].
    - REQ-1573 Establish agreements and procedures with entities involved in the supply chain for the system, system component, or system service for the [Selection (one or more): notification of supply chain compromises; [Assignment: organization-defined results of assessments or audits] ]. #low #moderate #high
  - ITEM-1027 SR-9 Tamper Resistance and Detection
    Related controls: [ITEM-0496], [ITEM-0588], [ITEM-0736], [ITEM-0918], [ITEM-0947], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1029], [ITEM-1030].
    - REQ-1574 Implement a tamper protection program for the system, system component, or system service. #high
    - ITEM-1028 SR-9(1) Multiple Stages of System Development Life Cycle
      Related controls: [ITEM-0658].
      - REQ-1575 Employ anti-tamper technologies, tools, and techniques throughout the system development life cycle. #high
  - ITEM-1029 SR-10 Inspection of Systems or Components
    Related controls: [ITEM-0142], [ITEM-0588], [ITEM-0918], [ITEM-0947], [ITEM-1011], [ITEM-1015], [ITEM-1020], [ITEM-1027], [ITEM-1030].
    - REQ-1576 Inspect the following systems or system components [Selection (one or more): at random; at [Assignment: organization-defined frequency] ; upon [Assignment: organization-defined indications of need for inspection] ] to detect tampering: [Assignment: organization-defined systems or system components]. #low #moderate #high
  - ITEM-1030 SR-11 Component Authenticity
    Related controls: [ITEM-0496], [ITEM-0662], [ITEM-0947], [ITEM-1027], [ITEM-1029].
    - REQ-1577 (a) Develop and implement anti-counterfeit policy and procedures that include the means to detect and prevent counterfeit components from entering the system; and #low #moderate #high
    - REQ-1578 (b) Report counterfeit system components to [Selection (one or more): source of counterfeit component; [Assignment: organization-defined external reporting organizations] ; [Assignment: organization-defined personnel or roles] ]. #low #moderate #high
    - ITEM-1031 SR-11(1) Anti-counterfeit Training
      Related controls: [ITEM-0142].
      - REQ-1579 Train [Assignment: organization-defined personnel or roles] to detect counterfeit system components (including hardware, software, and firmware). #low #moderate #high
    - ITEM-1032 SR-11(2) Configuration Control for Component Service and Repair
      Related controls: [ITEM-0239], [ITEM-0442], [ITEM-0451], [ITEM-0718].
      - REQ-1580 Maintain configuration control over the following system components awaiting service or repair and serviced or repaired components awaiting return to service: [Assignment: organization-defined system components]. #low #moderate #high
    - ITEM-1033 SR-11(3) Anti-counterfeit Scanning
      Related controls: [ITEM-0641].
      - REQ-1581 Scan for counterfeit system components [Assignment: organization-defined frequency].
  - ITEM-1034 SR-12 Component Disposal
    Related controls: [ITEM-0477].
    - REQ-1582 Dispose of [Assignment: organization-defined data, documentation, tools, or system components] using the following techniques and methods: [Assignment: organization-defined techniques and methods]. #low #moderate #high
