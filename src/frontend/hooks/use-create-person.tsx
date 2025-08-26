import { useMutation, useQueryClient } from "@tanstack/react-query";
import { backend } from "../../backend/declarations/index";

interface PersonInput {
  name: string;
  age: number;
}

export default function useCreatePerson() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (person: PersonInput) => {
      return backend.person_create(person);
    },
    onSuccess: () => {
      // Invalidate and refetch persons list after successful creation
      queryClient.invalidateQueries({ queryKey: ["query_persons"] });
    },
  });
}
