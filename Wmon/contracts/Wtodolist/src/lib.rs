#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype, Env, String, Symbol, Vec, Address, symbol_short
};

// --- TIPOS DE DATOS Y ERRORES ---

// Enum con los posibles estados de las tareas
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskStatus {
    Completed,
    Pending,
    Deleted,
}

// Estructura de una tarea, con 'owner' como Address
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub owner: Address, // CAMBIO: Usamos Address para la seguridad
    pub status: TaskStatus,
    pub timestamp: u64,
}

// Enum de errores personalizados
#[contracterror]
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TaskError {
    TaskNotFound = 1,
    InvalidTaskData = 2,
    Unauthorized = 3,       // NUEVO: Error de autorización
    TaskAlreadyCompleted = 4, // NUEVO: Error si se intenta completar una tarea ya completada
}

// --- CONTRATO Y CONSTANTES ---

#[contract]
pub struct ToDoListContract;

// Constante para la clave del próximo ID
const NEXT_ID_KEY: Symbol = symbol_short!("next_id");


// --- IMPLEMENTACIÓN DEL CONTRATO ---

#[contractimpl]
impl ToDoListContract {

    // 1. CREAR: Añadir nueva tarea 
    pub fn add_task(env: Env, description: String, owner: Address) -> Result<u32, TaskError> {
        // Validación de Seguridad: La dirección 'owner' debe firmar la transacción
        owner.require_auth(); 

        // Validar que la descripción no está vacía
        if description.len() == 0 {
            return Err(TaskError::InvalidTaskData);
        }
        
        // Obtener el próximo ID disponible
        let next_id = Self::get_next_task_id(&env);
        
        // Timestamp del bloque en epoch UNIX
        let timestamp: u64 = env.ledger().timestamp();

        let new_task = Task {
            id: next_id,
            description: description.clone(),
            owner: owner.clone(),
            status: TaskStatus::Pending,
            timestamp: timestamp,
        };

        // Guardar la tarea y actualizar el índice de IDs
        env.storage().instance().set(&next_id, &new_task);
        env.storage().instance().set(&NEXT_ID_KEY, &(next_id + 1));

        Ok(next_id)
    }

    // 2. LEER: Obtener tarea por ID
    // Función de solo lectura, no requiere auth
    pub fn get_task_by_id(env: Env, task_id: u32) -> Option<Task> {
        env.storage().instance().get(&task_id)
    }

    // 3. ACTUALIZAR: Concluir tarea
    pub fn task_completed(env: Env, task_id: u32, caller: Address) -> Result<(), TaskError> {
        // Seguridad: Solo el 'caller' (firmante) puede ejecutar esta función
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        // Validación: Solo el propietario original puede completar la tarea
        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }
        
        // Validación: No se puede completar una tarea ya completada
        if task.status == TaskStatus::Completed {
             return Err(TaskError::TaskAlreadyCompleted);
        }

        task.status = TaskStatus::Completed;

        // Guardar la tarea actualizada
        env.storage().instance().set(&task_id, &task);

        Ok(())
    }

    // 4. ACTUALIZAR (Soft Delete): Marcar tarea como eliminada
    pub fn task_deleted(env: Env, task_id: u32, caller: Address) -> Result<(), TaskError> {
        // Seguridad: Solo el 'caller' (firmante) puede ejecutar esta función
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        // Validación: Solo el propietario original puede marcar la tarea como eliminada
        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }

        task.status = TaskStatus::Deleted;

        // Guardar la tarea actualizada
        env.storage().instance().set(&task_id, &task);

        Ok(())
    }

    // --- NUEVAS FUNCIONES DE MANTENIMIENTO Y LECTURA AVANZADA ---

    // Función para cambiar el propietario (Transferencia de Tareas)
    pub fn transfer_ownership(env: Env, task_id: u32, caller: Address, new_owner: Address) -> Result<(), TaskError> {
        // Seguridad: Solo el 'caller' (propietario actual) debe firmar
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        // Validación: El caller debe ser el propietario actual
        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }

        task.owner = new_owner.clone();
        
        // Guardar la tarea con el nuevo propietario
        env.storage().instance().set(&task_id, &task);

        Ok(())
    }

    // Retorna todas las tareas pendientes y concluidas (excluye eliminadas)
    pub fn get_all(env: Env) -> Vec<Task> {
        let mut tasks = Vec::new(&env);
        let last_id = Self::get_next_task_id(&env);

        // Iterar las tareas y excluir las eliminadas
        for id in 1..last_id {
            if let Some(task) = Self::get_task_by_id(env.clone(), id) {
                if task.status != TaskStatus::Deleted { // Filtra las no-eliminadas
                    tasks.push_back(task);
                }
            }
        }
        tasks
    }


    /// Función helper para obtener el próximo ID disponible
    fn get_next_task_id(env: &Env) -> u32 {
        env.storage().instance().get(&NEXT_ID_KEY).unwrap_or(1)
    }
}
