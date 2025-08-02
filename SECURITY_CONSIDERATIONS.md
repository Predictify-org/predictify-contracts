#Security Considerations

This document highlights the important security concerns and protection strategies for the system.

## 1. Data Classification and it's Sensitivity
Implementing a data classification property
There are different types of data categories:
- Financial Data
- Intellectual Data
- Confidential Business Data
- Personal Identifiable Information
- Proprietary Data
- Health Information

Data classification based on sensitivity
- Public- Data can be accesed by public and free to access.
- Confidential - Data can accesed by authorized individuals, and highly protected data.
- Highly Confidential- Data is highly rescricted to acces and utmost confidentiality
- Internal: Data that can be shared only within an organization, which is private

## 2. Implementation of Encrytion

- Only authorized individuals can access data which is sensitive.
- Implementation of role-based access controls(RBAC) for permitting privileges based on roles and responsibilites.
- Implementating a multi-factor authentication(MFA) for adding extra layer of security for access control.
- For encrypting data during transmission, encryption mechanisms such as Secure Socket Layer(SSL) or Transport Layer Security(TLS) protocols can be added.

## 3. Data Encryption
- Transit Data Encrytion: Data moving from one point to another, with internet or VPN.

- Data Encryption at Rest: More secure and less data breach, less randsomeware attacks.

- Key Management Policy protects the sensitive data with processing cryptographic keys, with key generation and storage of keys.

## 4. Data Encrytion Algorithms
- The Advanced Encryption Standard(AES) is a symmetric-key algorithm. This employs block cipher methods.

- The latest version of the TLS protocol is TLS 1.3. Modern version of SSL, utilized by HTTPS and other protocols for encryption.

- Key rotation and retiring old keys with regularly updation of keys.

## 5. Security practices for third party dependencies
- Implementating tools like Synk or OWASP Dependency-Check with regularly scanning dependencies.

- Minimize risk posed by dependencies with isolation menthods: containerization, microservices architecture, and restricted permissions.

- Keeping updated libraries for security. Manual reviews are important for compatibility.

## 6. Regulatory Compilance
- HIPAA: Health Insurance Portability and Accountability Act maintains standards to protect sensitive health information from disclosure without patient's consent.

- GDPR: General Data Protection Regulation permits individuals the right to ask organisations to delete their personal data.