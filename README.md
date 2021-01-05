# khonsubase

## About this Project

Khonsubase is a lightweight project management and knowledgebase collaboration tool written in [rust](https://rust-lang.org/).

This project's aim is to provide an easy-to-self-host solution that requires few resources to run.

### State of the Project

This project was originally dreamed up years ago, but work only began in 2021. As such, it is not ready to be used by anyone. As of writing this, there isn't anything to use except a Login form. Once a link to an issue tracker for this project hosted on [base.khonsulabs.com](https://base.khonsulabs.com/) is available, then you might consider using it!

### The Background of Khonsubase

#### Why build another issue tracker?

Khonsubase is born out of the desires of the primary author, [Jon](https://github.com/ecton). As part of attempting to develop an MMO, it becomes overwhelming to organize task management for a codebase spread across multiple open source projects. I wanted a solution that could provide an issue tracker for anyone to use for my open-source projects, but also had powerful project management features that minimized manual issue management.

GitHub Issues is a great product because of how lightweight and easy it is. However, it doesn't support blocking issues (outside of manually doing it in the issue's description), and multi-project management is very lightweight. There are excellent solutions that don't offer public views into a subset of projects. And, there are a lot of solutions that aren't that great, but I won't name any.

As someone who ran his own tech team at a company he co-founded, Jon never really fell in love with any of the project management software that he tried, and most felt prohibitively expensive compared to the open-source software we were using at the time.

### Goals of Khonsubase

- Easy to deploy in a self-hosted environment
- Lightweight and performant
- Multilingual support
- Support Open and Closed source projects
- Support Confidential issue reporting
- Issue/Bug tracking with dependencies and unlimited hierarchy
- Personal task list management with private tasks and a unified Up Next view
- Markdown-based Knowledgebase

### What about feature X?

This project is being [dogfooded](https://en.wikipedia.org/wiki/Eating_your_own_dog_food) for [Khonsu Labs](https://khonsulabs.com/), and so a majority of the feature development will be based on those needs and wants. However, if you have a feature idea, don't hestitate to request it.

## Deploying Khonsubase

### Requirements

- PostgreSQL 11+ (Might work with previous versions, but is untested)
- (for attachments) Amazon S3-compatible Storage Bucket

### Installation Instructions (Ubuntu 20.04)

- Install libpq-dev:

  ```bash
  sudo apt install libpq-dev
  ```

- Create a user in postgres, e.g.:

  ```sql
  CREATE ROLE baseuser LOGIN PASSWORD 'SecretPassword' CONNECTION LIMIT -1;
  ```

- Create a database in postgres with the user created being the owner:

  ```sql
  CREATE DATABASE khonsubase OWNER baseuser;
  ```

- Build the server:

  ```bash
  cargo build --package server --release
  ```

- Copy the required files to your server:

  ```bash
  cp target/release/server deployment/khonsubase
  cp -r static/ deployment
  cp -r templates/ deployment
  ```

- Edit `.env` in your Deployment folder, or add these environment variables (update your database settings accordingly):

  ```bash
  DATABASE_URL="postgres://baseuser:SecretPassword@localhost/khonsubase"
  ```

- Launch the server to generate the default admin user:

  ```bash
  cd deployment;
  ./khonsubase
  ```

- Save the admin password. You can now set up a service to ensure it's always running. Here's an example systemd service (`/etc/systemd/system/khonsubase.service):

  ```ini
  [Unit]
  Description=Khonsubase Server
  After=network.target
  StartLimitIntervalSec=0

  [Service]
  Type=simple
  Restart=always
  RestartSec=1
  User=base
  WorkingDirectory=/home/base/deploy
  ExecStart=/home/base/deploy/khonsubase

  [Install]
  WantedBy=multi-user.target
  ```

  Once created, you can launch the service with `systemctl start khonsubase`. You can view logs using `journalctl -u khonsubase`. Enable the service at startup with `systemctl enable khonsubase

### Update Instructions

- Copy the new executable and updated `templates/` and `static` folders, and restart the server. All database migrations will be performed automatically, and any missing seed data will be initialized.
