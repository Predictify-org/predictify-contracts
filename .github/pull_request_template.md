# Pull Request Description

## 📋 Basic Information

### Type of Change
Please select the type of change this PR introduces:

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📚 Documentation update
- [ ] 🧪 Test addition/update
- [ ] 🔧 Refactoring (no functional changes)
- [ ] ⚡ Performance improvement
- [ ] 🔒 Security fix
- [ ] 🎨 UI/UX improvement
- [ ] 🚀 Deployment/Infrastructure change

### Related Issues
<!-- Link to related issues using keywords like "Closes", "Fixes", "Resolves" -->
Closes #(issue number)
Fixes #(issue number)
Related to #(issue number)

### Priority Level
- [ ] 🔴 Critical (blocking other development)
- [ ] 🟡 High (significant impact)
- [ ] 🟢 Medium (moderate impact)
- [ ] 🔵 Low (minor improvement)

---

## 📝 Detailed Description

### What does this PR do?
<!-- Provide a clear and concise description of what this PR accomplishes -->

### Why is this change needed?
<!-- Explain the problem this PR solves and why it's important -->

### How was this tested?
<!-- Describe the testing approach and results -->

### Alternative Solutions Considered
<!-- If applicable, describe other approaches you considered and why you chose this one -->

---

## 🏗️ Smart Contract Specific

### Contract Changes
Please check all that apply:

- [ ] Core contract logic modified
- [ ] Oracle integration changes (Pyth/Reflector)
- [ ] New functions added
- [ ] Existing functions modified
- [ ] Storage structure changes
- [ ] Events added/modified
- [ ] Error handling improved
- [ ] Gas optimization
- [ ] Access control changes
- [ ] Admin functions modified
- [ ] Fee structure changes

### Oracle Integration
- [ ] Pyth oracle integration affected
- [ ] Reflector oracle integration affected
- [ ] Oracle configuration changes
- [ ] Price feed handling modified
- [ ] Oracle fallback mechanisms
- [ ] Price validation logic

### Market Resolution Logic
- [ ] Hybrid resolution algorithm changed
- [ ] Dispute mechanism modified
- [ ] Fee structure updated
- [ ] Voting mechanism changes
- [ ] Community weight calculation
- [ ] Oracle weight calculation

### Security Considerations
- [ ] Access control reviewed
- [ ] Reentrancy protection
- [ ] Input validation
- [ ] Overflow/underflow protection
- [ ] Oracle manipulation protection

---

## 🧪 Testing

### Test Coverage
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests passing locally
- [ ] Manual testing completed
- [ ] Oracle integration tested
- [ ] Edge cases covered
- [ ] Error conditions tested
- [ ] Gas usage optimized
- [ ] Cross-contract interactions tested

### Test Results
```bash
# Paste test output here
cargo test
# Expected output: X tests passed, Y tests failed
```

### Manual Testing Steps
<!-- List the manual testing steps performed -->
1. 
2. 
3. 

---

## 📚 Documentation

### Documentation Updates
- [ ] README updated
- [ ] Code comments added/updated
- [ ] API documentation updated
- [ ] Examples updated
- [ ] Deployment instructions updated
- [ ] Contributing guidelines updated
- [ ] Architecture documentation updated

### Breaking Changes
<!-- If this PR includes breaking changes, describe them here -->
**Breaking Changes:**
- 
- 

**Migration Guide:**
<!-- If applicable, provide migration steps -->

---

## 🔍 Code Quality

### Code Review Checklist
- [ ] Code follows Rust/Soroban best practices
- [ ] Self-review completed
- [ ] No unnecessary code duplication
- [ ] Error handling is appropriate
- [ ] Logging/monitoring added where needed
- [ ] Security considerations addressed
- [ ] Performance implications considered
- [ ] Code is readable and well-commented
- [ ] Variable names are descriptive
- [ ] Functions are focused and small

### Performance Impact
<!-- Describe any performance implications of this change -->
- **Gas Usage**: 
- **Storage Impact**: 
- **Computational Complexity**: 

### Security Review
- [ ] No obvious security vulnerabilities
- [ ] Access controls properly implemented
- [ ] Input validation in place
- [ ] Oracle data properly validated
- [ ] No sensitive data exposed

---

## 🚀 Deployment & Integration

### Deployment Notes
<!-- Any special considerations for deployment -->
- **Network**: Testnet/Mainnet
- **Contract Address**: 
- **Migration Required**: Yes/No
- **Special Instructions**: 

### Integration Points
- [ ] Frontend integration considered
- [ ] API changes documented
- [ ] Backward compatibility maintained
- [ ] Third-party integrations updated

---

## 📊 Impact Assessment

### User Impact
<!-- How does this change affect end users? -->
- **End Users**: 
- **Developers**: 
- **Admins**: 

### Business Impact
<!-- How does this change affect the business/project? -->
- **Revenue**: 
- **User Experience**: 
- **Technical Debt**: 

---

## ✅ Final Checklist

### Pre-Submission
- [ ] Code follows Rust/Soroban best practices
- [ ] All CI checks passing
- [ ] No breaking changes (or breaking changes are documented)
- [ ] Ready for review
- [ ] PR description is complete and accurate
- [ ] All required sections filled out
- [ ] Test results included
- [ ] Documentation updated

### Review Readiness
- [ ] Self-review completed
- [ ] Code is clean and well-formatted
- [ ] Commit messages are clear and descriptive
- [ ] Branch is up to date with main
- [ ] No merge conflicts

---

## 📸 Screenshots (if applicable)

<!-- Add screenshots for UI changes or visual improvements -->

## 🔗 Additional Resources

<!-- Links to relevant documentation, discussions, design docs, etc. -->
- **Design Document**: 
- **Technical Spec**: 
- **Related Discussion**: 
- **External Documentation**: 

---

## 💬 Notes for Reviewers

<!-- Any specific areas you'd like reviewers to focus on -->
**Please pay special attention to:**
- 
- 

**Questions for reviewers:**
- 
- 

---

**Thank you for your contribution to Predictify! 🚀**