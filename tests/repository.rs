use pushkind_dantes::repository::DieselRepository;

mod common;

#[test]

fn test_user_repository_crud() {
    let test_db = common::TestDb::new();
    let _repo = DieselRepository::new(test_db.pool());
}
