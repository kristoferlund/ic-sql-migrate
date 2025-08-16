import { useMutation, useQueryClient } from "@tanstack/react-query";
import { backend } from "../../backend/declarations/index";

export default function useDeletePerson() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: bigint) => {
      return backend.delete(id);
    },
    onSuccess: () => {
      // Invalidate and refetch persons list after successful deletion
      queryClient.invalidateQueries({ queryKey: ["query_persons"] });
    },
  });
}
