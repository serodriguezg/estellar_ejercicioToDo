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
    pub owner: Address,
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
    Unauthorized = 3,
    TaskAlreadyCompleted = 4, // Usado también si se intenta modificar una tarea no-Pendiente
}

// --- CONTRATO Y CONSTANTES ---

#[contract]
pub struct ToDoListContract;

// Constante para la clave del próximo ID
const NEXT_ID_KEY: Symbol = symbol_short!("next_id");


// --- IMPLEMENTACIÓN DEL CONTRATO ---

#[contractimpl]
impl ToDoListContract {

    // 1. CREAR: Añadir nueva tarea y crear índice de propietario
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

        // 1. Guardar la tarea
        env.storage().instance().set(&next_id, &new_task);
        
        // 2. Indexación de tareas por Propietario (Address -> Vec<u32>)
        // La clave de almacenamiento es la Address del propietario
        let mut owner_tasks: Vec<u32> = env.storage().instance().get(&owner).unwrap_or(Vec::new(&env));
        owner_tasks.push_back(next_id);
        env.storage().instance().set(&owner, &owner_tasks);
        
        // 3. Actualizar el índice de IDs
        env.storage().instance().set(&NEXT_ID_KEY, &(next_id + 1));

        Ok(next_id)
    }

    // 2. LEER: Obtener tarea por ID
    pub fn get_task_by_id(env: Env, task_id: u32) -> Option<Task> {
        env.storage().instance().get(&task_id)
    }

    // 3. LEER AVANZADO: Retorna todas las tareas (no eliminadas) de un propietario específico
    // Esta función usa el índice que se creó en 'add_task'.
    pub fn get_tasks_by_owner(env: Env, owner: Address) -> Vec<Task> {
        let mut tasks = Vec::new(&env);
        
        // Intentar obtener la lista de IDs directamente desde la clave Address
        if let Some(task_ids) = env.storage().instance().get::<Address, Vec<u32>>(&owner) {
            
            // Iterar sobre los IDs indexados
            for task_id in task_ids.iter() {
                if let Some(task) = Self::get_task_by_id(env.clone(), task_id) {
                    // Solo incluir tareas que no estén marcadas como Deleted
                    if task.status != TaskStatus::Deleted {
                        tasks.push_back(task);
                    }
                }
            }
        }
        tasks
    }
    
    // 4. ACTUALIZAR: Concluir tarea
    pub fn task_completed(env: Env, task_id: u32, caller: Address) -> Result<(), TaskError> {
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }
        
        if task.status == TaskStatus::Completed {
             return Err(TaskError::TaskAlreadyCompleted);
        }

        task.status = TaskStatus::Completed;

        env.storage().instance().set(&task_id, &task);
        Ok(())
    }

    // 5. ACTUALIZAR: Modificar la descripción de una tarea pendiente (NUEVA FUNCIÓN)
    pub fn update_task_description(env: Env, task_id: u32, caller: Address, new_description: String) -> Result<(), TaskError> {
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        // Validación 1: Solo el propietario original
        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }
        
        // Validación 2: La nueva descripción no puede estar vacía
        if new_description.len() == 0 {
            return Err(TaskError::InvalidTaskData);
        }
        
        // Validación 3: Solo se pueden modificar tareas PENDIENTES
        if task.status != TaskStatus::Pending {
            return Err(TaskError::TaskAlreadyCompleted);
        }

        task.description = new_description.clone();

        env.storage().instance().set(&task_id, &task);
        Ok(())
    }

    // 6. ACTUALIZAR (Soft Delete): Marcar tarea como eliminada
    pub fn task_deleted(env: Env, task_id: u32, caller: Address) -> Result<(), TaskError> {
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }

        task.status = TaskStatus::Deleted;

        env.storage().instance().set(&task_id, &task);
        Ok(())
    }

    // 7. FUNCIÓN AVANZADA: Transferir Propiedad
    pub fn transfer_ownership(env: Env, task_id: u32, caller: Address, new_owner: Address) -> Result<(), TaskError> {
        // NOTA: Esta implementación NO actualiza los índices de propietario. 
        // Para tareas transferibles, un índice más complejo sería ideal.
        caller.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&task_id)
            .ok_or(TaskError::TaskNotFound)?;

        if task.owner != caller {
            return Err(TaskError::Unauthorized);
        }

        task.owner = new_owner.clone();
        
        env.storage().instance().set(&task_id, &task);
        Ok(())
    }

    // 8. LEER AVANZADO: Retorna todas las tareas pendientes y concluidas (excluye eliminadas)
    // NOTA: Esta función itera sobre todos los IDs, no es eficiente para contratos con muchos datos.
    pub fn get_all(env: Env) -> Vec<Task> {
        let mut tasks = Vec::new(&env);
        let last_id = Self::get_next_task_id(&env);

        for id in 1..last_id {
            if let Some(task) = Self::get_task_by_id(env.clone(), id) {
                if task.status != TaskStatus::Deleted {
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

// --- MÓDULO DE TESTS UNITARIOS ---

// Si usas este archivo como 'lib.rs', debes crear un archivo 'test.rs'
// o descomentar y completar este módulo para incluir todos los tests.

/*
#[cfg(test)]
mod test {
    use super::*; // Importar todo lo del scope superior
    // ... (Colocar aquí todos los tests que hemos generado) ...
}
*/