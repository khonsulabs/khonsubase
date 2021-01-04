# Development Notes

This is a working development notes document. It's really not meant to be readable by anyone except [@ecton](https://github.com/ecton).

## Tasks

- [ ] Need to suport account creation and login:

  - [ ] Create sign up form
  - [ ] Create sign up API error flow
  - [ ] Create sign up API positive flow
    - [ ] Create sessions table
    - [ ] Send cookie in response, redirect to home page
    - [ ] Update template to support knowing the user is logged in

- [ ] Need to support logout

- [ ] Need to support Log In

- [ ] Need Configuration System:

  - Environment Variables (or .env) for any values that impact launching the app
  - [ ] Rust configuration system
    - [x] Rust configuration system
    - [ ] Load configuration from Database
    - [ ] CRUD editing of configuration
      - Use Discourse's configuration admin as a model for how it should work.
      - Clear indicator of what is custom, and what the current value is
      - Easy way to clear an existing value back to the default value
    - [ ] Pub-sub for hot configuration reloads

- [ ] Need Wiki and CMS System:
  - [ ] Create database models
    - Need to store revisions, including the person saving
    - Need to support semver tagging
  - [ ] Seed Home page/Privacy Policy/TOS
  - [ ] Ensure we can update those pages and versioning works
  - [ ] Each page needs to be able to offer translated versions of the content (eventually)

### Things I will do soon-ish, but not on my immediate horizon

- [ ] 2-Factor auth support
- [ ] Add account login time restrictions to prevent brute-force password attacks
- [ ] Add account deletion
  - Deleting the user will not remove the user's account record, but instead it will sanitize the record.
    - Username should be replaced, and all mentions should be updated to the anonymized username
    - All session data will be removed
    - All private notes and tasks will be removed
- Do not allow the user to delete their account if they own any projects. They need to transfer ownership or delete the projects.

### Things I know I need to do but will probably put off for a while

The life of an indie developer.

- [ ] Add some sort of human-being challenge to account creation

## Pages Needed for Something

Required pages:

- [x] Home Page
- [ ] TOS
- [ ] Privacy Policy
- [ ] Login
  - Fields:
    - Username
    - Password
- [ ] Sign up:
  - Fields:
    - Username
    - Password
    - Password Verify
    - Accept TOS (Special page)
