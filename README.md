# Cliente do Sistema de Chave-Valor Distribuído

Este projeto contém o cliente gRPC para interagir com o **Servidor de Chave-Valor Distribuído**, que você implementou ou está implementando como parte do seu trabalho. Ele permite que você envie operações de **`Put`** (escrever dados) e **`Get`** (ler dados) para qualquer nó do seu cluster de K-V Store.

---

## Funcionamento

O cliente se comunica com um nó específico do servidor usando **gRPC**. Ele recebe o endereço do servidor como um argumento de linha de comando e, em seguida, executa a operação solicitada (`put` ou `get`).

---

## Estrutura do Projeto

* `proto/kv_store.proto`: O arquivo de definição do serviço gRPC. **Ele é a "linguagem" que o cliente e o servidor usam para conversar.**
* `src/main.rs`: O código-fonte principal do cliente em Rust. (Este cliente é implementado em Rust, mas o conceito é o mesmo para qualquer linguagem que você usar.)

---

## Pré-requisitos

Para usar este cliente, você precisará ter:

* **Rust e Cargo:** Se você ainda não os tem, pode instalá-los através do [rustup](https://rustup.rs/).
* Uma ou mais instâncias do seu **servidor de Chave-Valor Distribuído** em execução, escutando em um endereço conhecido (e um **broker MQTT** também ativo, pois o servidor precisa dele).

---

## Como Compilar e Executar

Siga os passos abaixo para compilar e interagir com o seu servidor.

### 1. Compilar o Projeto

Execute o script `compile.sh` ou navegue até a raiz do seu projeto (onde está o `Cargo.toml` principal) e compile o cliente:

```bash
cargo build
```

Isso compilará tanto o servidor quanto o cliente. O executável do cliente estará em `target/debug/client`.

### 2. Executar Operações

Você pode executar operações de `put` e `get` fornecendo o endereço do servidor e os detalhes da operação.

#### Operação `put`

Para armazenar um valor para uma chave:

```bash
cargo run -- <ENDERECO_DO_SERVIDOR> put <CHAVE> <VALOR>

# ou usando o script
./client.sh <ENDERECO_DO_SERVIDOR> put <CHAVE> <VALOR>
```


**Exemplo:**

Se seu servidor estiver rodando em `127.0.0.1:50051`:

```bash
cargo run -- 127.0.0.1:50051 put meu_nome "Maria Silva"

# ou usando o script
./client.sh 127.0.0.1:50051 put meu_nome "Maria Silva"
```


#### Operação `get`

Para recuperar valores para uma chave:

```bash
cargo run -- <ENDERECO_DO_SERVIDOR> get <CHAVE>
 
# ou usando o script
./client.sh <ENDERECO_DO_SERVIDOR> get <CHAVE>
```

**Exemplo:**

Para obter o valor da chave `meu_nome` do servidor em `127.0.0.1:50051`:

```bash
cargo run -- 127.0.0.1:50051 get meu_nome

# ou usando o script
./client.sh 127.0.0.1:50051 get meu_nome
```

Se houver múltiplas **versões** ativas para a chave (devido a **concorrência**), o cliente as exibirá.

---

## Exemplo Completo de Uso (Cenário Distribuído)

Para ver a **replicação** e **resolução de conflitos** em ação, você precisará de múltiplos servidores rodando.

1.  **Inicie um servidor (Nó A):**

    ```bash
    # No terminal 1
    ./server.sh --node-id node_A --listen-addr 127.0.0.1:50051 --mqtt-broker-addr 127.0.0.1 --mqtt-broker-port 1883
    ```

2.  **Inicie outro servidor (Nó B):**

    ```bash
    # No terminal 2
    ./server.sh --node-id node_B --listen-addr 127.0.0.1:50052 --mqtt-broker-addr 127.0.0.1 --mqtt-broker-port 1883
    ```

3.  **Adicione um valor ao Nó A usando o cliente:**

    ```bash
    # No terminal 3 (ou em qualquer outro terminal)
    ./client.sh 127.0.0.1:50051 put produto_1 "notebook_azul"
    ```

    Você deve ver `node_B` recebendo a replicação do `produto_1`.

4.  **Recupere o valor do Nó B:**

    ```bash
    ./client.sh 127.0.0.1:50052 get produto_1
    ```

    Mesmo tendo feito o `put` no Nó A, o Nó B deve retornar "notebook_azul" devido à replicação via MQTT.

5.  **Simule um conflito:**

    * Faça um `put` no Nó A:

        ```bash
        ./client.sh 127.0.0.1:50051 put item_conflito "valor_original"
        ```

    * **Rapidamente**, antes que o Nó B receba a replicação, faça um `put` da *mesma chave* no Nó B com um valor diferente:

        ```bash
        ./client.sh 127.0.0.1:50052 put item_conflito "novo_valor_conflitante"
        ```

    * Agora, após a replicação se estabilizar, tente `get` de qualquer um dos nós:

        ```bash
        ./client.sh 127.0.0.1:50051 get item_conflito
        # OU
        ./client.sh 127.0.0.1:50052 get item_conflito
        ```

        Você deve ver **ambas** as versões (`valor_original` e `novo_valor_conflitante`) retornadas, pois elas são consideradas **concorrentes** pelos **Vector Clocks**!

Este cliente é uma ferramenta fundamental para testar e entender o comportamento distribuído do seu sistema de Chave-Valor. 
Experimente diferentes cenários e observe como a consistência eventual e a resolução de conflitos funcionam na prática!
