from bcdb import Identity, Client, HTTPClient

if __name__ == "__main__":
    identity = Identity.from_seed("user.seed")
    client = HTTPClient("http://localhost:3030", identity)
    collection = client.collection("http")

    id = collection.set(
        "Some data goes here", {"name": "test", "parent": "some parent with\nnewlines"}, acl='10')

    print(id)

    response = collection.get(id)

    print(response)
    print(response.headers)
    print(response.text)

    print("update", id)
    response = collection.update(id,
                                 "Updated data", {"name": "test", "parent": "new value"})

    print("Update Response:", response)

    response = collection.get(id)

    print(response)
    print(response.headers)
    print(response.text)

    acl = client.acl

    response = acl.create("r--", [1])

    print(response)
    print(response.headers)
    print(response.text)

    response = acl.get("0")

    print(response)
    print(response.headers)
    print(response.text)

    response = acl.set("0", "rwd")

    print(response)
    print(response.headers)
    print(response.text)

    response = acl.grant("0", [1])

    print(response)
    print(response.headers)
    print(response.text)

    response = acl.revoke("0", [1])

    print(response)
    print(response.headers)
    print(response.text)

    response = acl.list()

    print(response)
    print(response.headers)
    print(response.text)
