#[cfg(test)]
mod test;
// --- Requerido para simular firmas de direcciones ---
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Env, String, Address,
};

// Importar el contrato y las estructuras
use crate::{ToDoListContract, ToDoListContractClient, Task, TaskStatus, TaskError, symbol_short};


// Función de configuración común para los tests
fn setup_env() -> (Env, ToDoListContractClient<'static>, Address, Address) {
    let env = Env::default();
    // Aumentar el tiempo del ledger para el timestamp de la tarea
    env.ledger().set_timestamp(1678886400); // 15 de marzo de 2023, 00:00:00 UTC
    
    let contract_id = env.register_contract(None, ToDoListContract);
    let client = ToDoListContractClient::new(&env, &contract_id);
    
    // Crear direcciones simuladas para el propietario y otro usuario
    let owner_a = Address::random(&env);
    let owner_b = Address::random(&env);

    (env, client, owner_a, owner_b)
}

// =======================================================
// TEST: add_task
// =======================================================

#[test]
fn test_add_task_success() {
    let (env, client, owner_a, _) = setup_env();
    let desc = String::from_str(&env, "Comprar leche");

    // Llama a add_task. Se simula que 'owner_a' firma la transacción.
    let task_id = client.add_task(&desc, &owner_a);
    
    // El ID de la primera tarea debe ser 1
    assert_eq!(task_id, 1);

    // Verificar que la tarea se guardó correctamente
    let task = client.get_task_by_id(&task_id).unwrap();
    
    assert_eq!(task.id, 1);
    assert_eq!(task.description, desc);
    assert_eq!(task.owner, owner_a);
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.timestamp, env.ledger().timestamp());

    // Verificar que el próximo ID se incrementó
    let next_id_key = symbol_short!("next_id");
    let next_id: u32 = env.storage().instance().get(&next_id_key).unwrap();
    assert_eq!(next_id, 2);
}

#[test]
fn test_add_task_empty_description_fails() {
    let (env, client, owner_a, _) = setup_env();
    let desc_empty = String::from_str(&env, "");

    // Debe fallar con InvalidTaskData
    let result = client.try_add_task(&desc_empty, &owner_a);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::InvalidTaskData);
}

// =======================================================
// TEST: get_task_by_id
// =======================================================

#[test]
fn test_get_task_by_id_success() {
    let (env, client, owner_a, _) = setup_env();
    let desc = String::from_str(&env, "Hacer ejercicio");
    let task_id = client.add_task(&desc, &owner_a);

    let retrieved_task = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(retrieved_task.id, task_id);
    assert_eq!(retrieved_task.status, TaskStatus::Pending);
}

#[test]
fn test_get_task_by_id_not_found() {
    let (_env, client, _, _) = setup_env();
    // Intenta obtener una tarea que no existe (ej. ID 99)
    let retrieved_task = client.get_task_by_id(&99);
    assert!(retrieved_task.is_none());
}

// =======================================================
// TEST: task_completed
// =======================================================

#[test]
fn test_task_completed_success() {
    let (env, client, owner_a, _) = setup_env();
    let desc = String::from_str(&env, "Pagar facturas");
    let task_id = client.add_task(&desc, &owner_a);

    // Simula que 'owner_a' completa su tarea
    client.task_completed(&task_id, &owner_a);

    // Verificar el estado
    let task = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
}

#[test]
fn test_task_completed_unauthorized_fails() {
    let (env, client, owner_a, other_user) = setup_env();
    let desc = String::from_str(&env, "Revisar código");
    let task_id = client.add_task(&desc, &owner_a);

    // Simula que 'other_user' intenta completar la tarea de 'owner_a'
    let result = client.try_task_completed(&task_id, &other_user);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::Unauthorized);
}

#[test]
fn test_task_completed_not_found_fails() {
    let (_env, client, owner_a, _) = setup_env();

    // Intenta completar una tarea que no existe
    let result = client.try_task_completed(&99, &owner_a);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::TaskNotFound);
}

#[test]
fn test_task_completed_already_completed_fails() {
    let (env, client, owner_a, _) = setup_env();
    let desc = String::from_str(&env, "Ir al supermercado");
    let task_id = client.add_task(&desc, &owner_a);

    // 1. Completar la tarea
    client.task_completed(&task_id, &owner_a);

    // 2. Intentar completarla de nuevo
    let result = client.try_task_completed(&task_id, &owner_a);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::TaskAlreadyCompleted);
}

// =======================================================
// TEST: task_deleted
// =======================================================

#[test]
fn test_task_deleted_success() {
    let (env, client, owner_a, _) = setup_env();
    let desc = String::from_str(&env, "Hacer soft delete test");
    let task_id = client.add_task(&desc, &owner_a);

    // Simula que 'owner_a' marca como eliminada su tarea
    client.task_deleted(&task_id, &owner_a);

    // Verificar el estado
    let task = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Deleted);
}

#[test]
fn test_task_deleted_unauthorized_fails() {
    let (env, client, owner_a, other_user) = setup_env();
    let desc = String::from_str(&env, "Soft delete protection test");
    let task_id = client.add_task(&desc, &owner_a);

    // Simula que 'other_user' intenta eliminar la tarea de 'owner_a'
    let result = client.try_task_deleted(&task_id, &other_user);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::Unauthorized);
}

// =======================================================
// TEST: transfer_ownership
// =======================================================

#[test]
fn test_transfer_ownership_success() {
    let (env, client, owner_a, new_owner) = setup_env();
    let desc = String::from_str(&env, "Delegar trabajo");
    let task_id = client.add_task(&desc, &owner_a);

    // 'owner_a' transfiere la propiedad a 'new_owner'
    client.transfer_ownership(&task_id, &owner_a, &new_owner);

    // Verificar el nuevo propietario
    let task = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(task.owner, new_owner);

    // Verificar que el 'new_owner' ahora puede completar la tarea
    client.task_completed(&task_id, &new_owner);
    let task_completed = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(task_completed.status, TaskStatus::Completed);
}

#[test]
fn test_transfer_ownership_unauthorized_fails() {
    let (env, client, owner_a, other_user) = setup_env();
    let new_owner = Address::random(&env);
    let desc = String::from_str(&env, "Intento de transferencia no autorizada");
    let task_id = client.add_task(&desc, &owner_a);

    // 'other_user' intenta transferir la tarea de 'owner_a' a 'new_owner'
    let result = client.try_transfer_ownership(&task_id, &other_user, &new_owner);
    assert_eq!(result.err().unwrap().unwrap(), TaskError::Unauthorized);

    // Verificar que el propietario sigue siendo el original
    let task = client.get_task_by_id(&task_id).unwrap();
    assert_eq!(task.owner, owner_a);
}

// =======================================================
// TEST: get_all
// =======================================================

#[test]
fn test_get_all_filters_deleted() {
    let (env, client, owner_a, owner_b) = setup_env();

    // Tarea 1: Pending (owner_a)
    client.add_task(&String::from_str(&env, "T1"), &owner_a);
    
    // Tarea 2: Completed (owner_a)
    let t2_id = client.add_task(&String::from_str(&env, "T2"), &owner_a);
    client.task_completed(&t2_id, &owner_a);

    // Tarea 3: Deleted (owner_b)
    let t3_id = client.add_task(&String::from_str(&env, "T3"), &owner_b);
    client.task_deleted(&t3_id, &owner_b);
    
    // Tarea 4: Pending (owner_b)
    client.add_task(&String::from_str(&env, "T4"), &owner_b);


    let all_tasks = client.get_all();

    // Solo se deben retornar T1, T2 y T4 (3 tareas)
    assert_eq!(all_tasks.len(), 3);

    // Verificar que T3 (ID 3) no está presente
    let t3_present = all_tasks.iter().any(|t| t.id == 3);
    assert!(!t3_present);

    // Verificar que las tareas restantes son las correctas
    let task_ids: Vec<u32> = all_tasks.iter().map(|t| t.id).collect();
    assert_eq!(task_ids, vec![1, 2, 4]);
}

#[test]
fn test_get_all_empty() {
    let (_env, client, _, _) = setup_env();
    
    let all_tasks = client.get_all();
    
    // Debe retornar un Vec vacío
    assert!(all_tasks.is_empty());
}