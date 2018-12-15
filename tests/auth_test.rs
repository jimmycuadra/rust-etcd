use etcd::auth::{self, AuthChange, NewUser, Role, RoleUpdate, UserUpdate};
use etcd::{BasicAuth, Client};
use futures::future::Future;
use tokio::runtime::Runtime;

#[test]
fn auth() {
    let client = Client::new(&["http://etcd:2379"], None).unwrap();
    let client_2 = client.clone();
    let client_3 = client.clone();

    let basic_auth = BasicAuth {
        username: "root".into(),
        password: "secret".into(),
    };

    let authed_client = Client::new(&["http://etcd:2379"], Some(basic_auth)).unwrap();
    let authed_client_2 = authed_client.clone();
    let authed_client_3 = authed_client.clone();
    let authed_client_4 = authed_client.clone();
    let authed_client_5 = authed_client.clone();
    let authed_client_6 = authed_client.clone();
    let authed_client_7 = authed_client.clone();
    let authed_client_8 = authed_client.clone();
    let authed_client_9 = authed_client.clone();

    let root_user = NewUser::new("root", "secret");

    let work: Box<dyn Future<Item = (), Error = ()> + Send> = Box::new(
        auth::status(&client)
            .then(move |res| {
                let response = res.unwrap();

                assert_eq!(response.data, false);

                auth::create_user(&client_2, root_user)
            })
            .then(move |res| {
                let response = res.unwrap();

                assert_eq!(response.data.name(), "root");

                auth::enable(&client_3)
            })
            .then(move |res| {
                let response = res.unwrap();

                assert_eq!(response.data, AuthChange::Changed);

                let mut update_guest = RoleUpdate::new("guest");

                update_guest.revoke_kv_write_permission("/*");

                auth::update_role(&authed_client, update_guest)
            })
            .then(move |res| {
                res.unwrap();

                let mut rkt_role = Role::new("rkt");

                rkt_role.grant_kv_read_permission("/rkt/*");
                rkt_role.grant_kv_write_permission("/rkt/*");

                auth::create_role(&authed_client_2, rkt_role)
            })
            .then(move |res| {
                res.unwrap();

                let mut rkt_user = NewUser::new("rkt", "secret");

                rkt_user.add_role("rkt");

                auth::create_user(&authed_client_3, rkt_user)
            })
            .then(move |res| {
                let response = res.unwrap();

                let rkt_user = response.data;

                assert_eq!(rkt_user.name(), "rkt");

                let role_name = &rkt_user.role_names()[0];

                assert_eq!(role_name, "rkt");

                let mut update_rkt_user = UserUpdate::new("rkt");

                update_rkt_user.update_password("secret2");
                update_rkt_user.grant_role("root");

                auth::update_user(&authed_client_4, update_rkt_user)
            })
            .then(move |res| {
                res.unwrap();

                auth::get_role(&authed_client_5, "rkt")
            })
            .then(move |res| {
                let response = res.unwrap();

                let role = response.data;

                assert!(role.kv_read_permissions().contains(&"/rkt/*".to_owned()));
                assert!(role.kv_write_permissions().contains(&"/rkt/*".to_owned()));

                auth::delete_user(&authed_client_6, "rkt")
            })
            .then(move |res| {
                res.unwrap();

                auth::delete_role(&authed_client_7, "rkt")
            })
            .then(move |res| {
                res.unwrap();

                let mut update_guest = RoleUpdate::new("guest");

                update_guest.grant_kv_write_permission("/*");

                auth::update_role(&authed_client_8, update_guest)
            })
            .then(move |res| {
                res.unwrap();

                auth::disable(&authed_client_9)
            })
            .then(|res| {
                let response = res.unwrap();

                assert_eq!(response.data, AuthChange::Changed);

                Ok(())
            }),
    );

    let _ = Runtime::new()
        .expect("failed to create Tokio runtime")
        .block_on(work);
}
