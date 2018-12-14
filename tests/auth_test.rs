extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_timer;

use etcd::auth::{self, AuthChange, NewUser, Role, RoleUpdate, UserUpdate};
use etcd::{BasicAuth, Client};
use futures::future::Future;
use tokio_core::reactor::Core;

#[test]
fn auth() {
    let mut core = Core::new().unwrap();
    let client = Client::new(&["http://etcd:2379"], None).unwrap();

    let basic_auth = BasicAuth {
        username: "root".into(),
        password: "secret".into(),
    };

    let authed_client = Client::new(&["http://etcd:2379"], Some(basic_auth)).unwrap();

    let root_user = NewUser::new("root", "secret");

    let work: Box<Future<Item = (), Error = ()>> = Box::new(
        auth::status(&client)
            .then(|res| {
                let response = res.unwrap();

                assert_eq!(response.data, false);

                auth::create_user(&client, root_user)
            })
            .then(|res| {
                let response = res.unwrap();

                assert_eq!(response.data.name(), "root");

                auth::enable(&client)
            })
            .then(|res| {
                let response = res.unwrap();

                assert_eq!(response.data, AuthChange::Changed);

                let mut update_guest = RoleUpdate::new("guest");

                update_guest.revoke_kv_write_permission("/*");

                auth::update_role(&authed_client, update_guest)
            })
            .then(|res| {
                res.unwrap();

                let mut rkt_role = Role::new("rkt");

                rkt_role.grant_kv_read_permission("/rkt/*");
                rkt_role.grant_kv_write_permission("/rkt/*");

                auth::create_role(&authed_client, rkt_role)
            })
            .then(|res| {
                res.unwrap();

                let mut rkt_user = NewUser::new("rkt", "secret");

                rkt_user.add_role("rkt");

                auth::create_user(&authed_client, rkt_user)
            })
            .then(|res| {
                let response = res.unwrap();

                let rkt_user = response.data;

                assert_eq!(rkt_user.name(), "rkt");

                let role_name = &rkt_user.role_names()[0];

                assert_eq!(role_name, "rkt");

                let mut update_rkt_user = UserUpdate::new("rkt");

                update_rkt_user.update_password("secret2");
                update_rkt_user.grant_role("root");

                auth::update_user(&authed_client, update_rkt_user)
            })
            .then(|res| {
                res.unwrap();

                auth::get_role(&authed_client, "rkt")
            })
            .then(|res| {
                let response = res.unwrap();

                let role = response.data;

                assert!(role.kv_read_permissions().contains(&"/rkt/*".to_owned()));
                assert!(role.kv_write_permissions().contains(&"/rkt/*".to_owned()));

                auth::delete_user(&authed_client, "rkt")
            })
            .then(|res| {
                res.unwrap();

                auth::delete_role(&authed_client, "rkt")
            })
            .then(|res| {
                res.unwrap();

                let mut update_guest = RoleUpdate::new("guest");

                update_guest.grant_kv_write_permission("/*");

                auth::update_role(&authed_client, update_guest)
            })
            .then(|res| {
                res.unwrap();

                auth::disable(&authed_client)
            })
            .then(|res| {
                let response = res.unwrap();

                assert_eq!(response.data, AuthChange::Changed);

                Ok(())
            }),
    );

    assert!(core.run(work).is_ok());
}
