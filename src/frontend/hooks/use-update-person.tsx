import { useMutation, useQueryClient } from "@tanstack/react-query";
import { backend } from "../../backend/declarations/index";

interface UpdatePersonInput {
  id: number;
  name: string;
}

export default function useUpdatePerson() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: UpdatePersonInput) => {
      return backend.person_update(params);
    },
    onSuccess: () => {
      // Invalidate and refetch persons list after successful update
      queryClient.invalidateQueries({ queryKey: ["query_persons"] });
    },
  });
}
