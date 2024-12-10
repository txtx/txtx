```hcl
flow "sepolia" {

}
flow "op" {
    
}

runbook "runbook1" {
    ...
}

action "sign_tx" {
    ...
}

action "sign_tx" {
    ...
}

action "sign_tx" {
    ...
}

runbook "runbook2" {
    ...
}

```

```hcl - runbook1
action "stuff" {
    ...
}
runbook "runbook3" {
    input = action.stuff.result
}

```


IndexSet = ["runbook0", "runbook1", "runbook2", "runbook3"]
```


